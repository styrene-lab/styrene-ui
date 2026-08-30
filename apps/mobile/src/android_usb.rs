use async_channel::{Receiver, Sender};
use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JObject, JString, JValue},
};
use std::sync::Arc;
use styrene_ui_platform::{
    AndroidUsbAttachment, AndroidUsbByteLink, PlatformFailure, PlatformFuture,
};

const USB_QUEUE_CAPACITY: usize = 8;
const USB_READ_BYTES_JNI: i32 = 4_096;
const USB_READ_TIMEOUT_MS: i32 = 100;
const USB_WRITE_TIMEOUT_MS: i32 = 1_000;
const USB_WRITE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(2);
const USB_MAX_WRITE_BYTES: usize = 4_096;
const USB_ENDPOINT_XFER_BULK: i32 = 2;
const USB_DIR_IN: i32 = 0x80;
const USB_CLASS_COMM: i32 = 2;
const USB_CLASS_CDC_DATA: i32 = 10;
const USB_CLASS_VENDOR_SPEC: i32 = 255;
const SILABS_VENDOR_ID: i32 = 0x10c4;

enum UsbCommand {
    Write { data: Vec<u8>, response: Sender<Result<(), PlatformFailure>> },
    Close,
}

struct NativeUsbPort {
    vm: Arc<JavaVM>,
    connection: GlobalRef,
    data_interface: GlobalRef,
    control_interface: Option<GlobalRef>,
    read_endpoint: GlobalRef,
    write_endpoint: GlobalRef,
}

impl Drop for NativeUsbPort {
    fn drop(&mut self) {
        if let Ok(mut env) = self.vm.attach_current_thread() {
            close_native(&mut env, self);
        }
    }
}

pub struct AndroidUsbLink {
    commands: Sender<UsbCommand>,
    reads: Receiver<Result<Vec<u8>, PlatformFailure>>,
    worker: Option<std::thread::JoinHandle<()>>,
    closed: bool,
}

impl AndroidUsbLink {
    pub const MAX_WRITE_SIZE: usize = USB_MAX_WRITE_BYTES;

    pub async fn open(attachment: AndroidUsbAttachment) -> Result<Self, PlatformFailure> {
        let native =
            dispatch_query(move |env, activity| open_native(env, activity, &attachment)).await?;
        let (commands, command_rx) = async_channel::bounded(USB_QUEUE_CAPACITY);
        let (read_tx, reads) = async_channel::bounded(USB_QUEUE_CAPACITY);
        let worker = std::thread::Builder::new()
            .name("styrene-android-usb".into())
            .spawn(move || run_worker(&native, &command_rx, &read_tx))
            .map_err(|_| failure("android_usb_worker_start_failed", true))?;
        Ok(Self { commands, reads, worker: Some(worker), closed: false })
    }

    pub async fn read(&self) -> Result<Option<Vec<u8>>, PlatformFailure> {
        match tokio::time::timeout(std::time::Duration::from_millis(150), self.reads.recv()).await {
            Err(_) => Ok(None),
            Ok(Ok(result)) => result.map(Some),
            Ok(Err(_)) => Err(failure("android_usb_worker_closed", true)),
        }
    }

    pub async fn write(&self, data: Vec<u8>) -> Result<(), PlatformFailure> {
        if data.len() > USB_MAX_WRITE_BYTES {
            return Err(failure("android_usb_write_too_large", false));
        }
        let (response, result) = async_channel::bounded(1);
        self.commands
            .send(UsbCommand::Write { data, response })
            .await
            .map_err(|_| failure("android_usb_worker_closed", true))?;
        tokio::time::timeout(std::time::Duration::from_secs(2), result.recv())
            .await
            .map_err(|_| failure("android_usb_write_timeout", true))?
            .map_err(|_| failure("android_usb_write_result_closed", true))?
    }

    pub fn close(&mut self) {
        if !self.closed {
            self.closed = true;
            let _ = self.commands.try_send(UsbCommand::Close);
            self.commands.close();
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }
}

impl Drop for AndroidUsbLink {
    fn drop(&mut self) {
        self.close();
    }
}

impl AndroidUsbByteLink for AndroidUsbLink {
    fn read(&self) -> PlatformFuture<'_, Result<Option<Vec<u8>>, PlatformFailure>> {
        Box::pin(AndroidUsbLink::read(self))
    }

    fn write(&self, data: Vec<u8>) -> PlatformFuture<'_, Result<(), PlatformFailure>> {
        Box::pin(AndroidUsbLink::write(self, data))
    }

    fn close(&mut self) {
        AndroidUsbLink::close(self);
    }
}

fn run_worker(
    native: &NativeUsbPort,
    commands: &Receiver<UsbCommand>,
    reads: &Sender<Result<Vec<u8>, PlatformFailure>>,
) {
    let Ok(mut env) = native.vm.attach_current_thread_permanently() else {
        let _ = reads.force_send(Err(failure("android_usb_thread_attach_failed", false)));
        return;
    };
    loop {
        match commands.try_recv() {
            Ok(UsbCommand::Write { data, response }) => {
                let result = env
                    .with_local_frame(16, |env| write_all(env, native, &data))
                    .map_err(|_| clear_jni_failure(&mut env, "android_usb_write_failed", true));
                let failed = result.is_err();
                let _ = response.try_send(result);
                if failed {
                    break;
                }
                continue;
            }
            Ok(UsbCommand::Close) | Err(async_channel::TryRecvError::Closed) => break,
            Err(async_channel::TryRecvError::Empty) => {}
        }

        match env.with_local_frame(8, |env| read_once(env, native)) {
            Ok(Some(bytes)) => {
                if reads.try_send(Ok(bytes)).is_err() {
                    let _ = reads.force_send(Err(failure("android_usb_read_queue_full", false)));
                    break;
                }
            }
            Ok(None) => {}
            Err(_) => {
                let error = clear_jni_failure(&mut env, "android_usb_read_failed", true);
                let _ = reads.force_send(Err(error));
                break;
            }
        }
    }
}

fn read_once(env: &mut JNIEnv<'_>, native: &NativeUsbPort) -> jni::errors::Result<Option<Vec<u8>>> {
    let buffer = env.new_byte_array(USB_READ_BYTES_JNI)?;
    let count = env
        .call_method(
            native.connection.as_obj(),
            "bulkTransfer",
            "(Landroid/hardware/usb/UsbEndpoint;[BII)I",
            &[
                JValue::Object(native.read_endpoint.as_obj()),
                JValue::Object(&buffer),
                JValue::Int(USB_READ_BYTES_JNI),
                JValue::Int(USB_READ_TIMEOUT_MS),
            ],
        )?
        .i()?;
    if count <= 0 {
        return Ok(None);
    }
    let mut bytes = env.convert_byte_array(&buffer)?;
    bytes.truncate(usize::try_from(count).map_err(|_| invalid_arguments())?);
    Ok(Some(bytes))
}

fn write_all(env: &mut JNIEnv<'_>, native: &NativeUsbPort, data: &[u8]) -> jni::errors::Result<()> {
    let deadline = std::time::Instant::now() + USB_WRITE_DEADLINE;
    let mut offset = 0;
    while offset < data.len() {
        let timeout = deadline.saturating_duration_since(std::time::Instant::now());
        if timeout.is_zero() {
            return Err(invalid_arguments());
        }
        let timeout_ms = i32::try_from(timeout.as_millis().min(USB_WRITE_TIMEOUT_MS as u128))
            .map_err(|_| invalid_arguments())?
            .max(1);
        let remaining = &data[offset..];
        let buffer = env.byte_array_from_slice(remaining)?;
        let written = env
            .call_method(
                native.connection.as_obj(),
                "bulkTransfer",
                "(Landroid/hardware/usb/UsbEndpoint;[BII)I",
                &[
                    JValue::Object(native.write_endpoint.as_obj()),
                    JValue::Object(&buffer),
                    JValue::Int(jni_len(remaining.len())?),
                    JValue::Int(timeout_ms),
                ],
            )?
            .i()?;
        let written = usize::try_from(written).map_err(|_| invalid_arguments())?;
        if written == 0 || written > remaining.len() {
            return Err(invalid_arguments());
        }
        offset += written;
    }
    Ok(())
}

fn close_native(env: &mut JNIEnv<'_>, native: &NativeUsbPort) {
    let _ = env.call_method(
        native.connection.as_obj(),
        "releaseInterface",
        "(Landroid/hardware/usb/UsbInterface;)Z",
        &[JValue::Object(native.data_interface.as_obj())],
    );
    if let Some(control) = native.control_interface.as_ref() {
        let _ = env.call_method(
            native.connection.as_obj(),
            "releaseInterface",
            "(Landroid/hardware/usb/UsbInterface;)Z",
            &[JValue::Object(control.as_obj())],
        );
    }
    let _ = env.call_method(native.connection.as_obj(), "close", "()V", &[]);
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
}

fn open_native(
    env: &mut JNIEnv<'_>,
    activity: &JObject<'_>,
    expected: &AndroidUsbAttachment,
) -> jni::errors::Result<NativeUsbPort> {
    let vm = Arc::new(env.get_java_vm()?);
    let manager = usb_manager(env, activity)?;
    let device = find_device(env, &manager, expected)?.ok_or_else(invalid_arguments)?;
    let authorized = env
        .call_method(
            &manager,
            "hasPermission",
            "(Landroid/hardware/usb/UsbDevice;)Z",
            &[JValue::Object(&device)],
        )?
        .z()?;
    if !authorized {
        return Err(invalid_arguments());
    }
    let selection = select_native_port(env, &device, expected.vendor_id)?;
    let connection = env
        .call_method(
            &manager,
            "openDevice",
            "(Landroid/hardware/usb/UsbDevice;)Landroid/hardware/usb/UsbDeviceConnection;",
            &[JValue::Object(&device)],
        )?
        .l()?;
    if connection.is_null() {
        return Err(invalid_arguments());
    }

    if let Some(control) = selection.control_interface.as_ref()
        && let Err(error) = claim_interface(env, &connection, control)
    {
        clear_pending_exception(env);
        close_open_connection(env, &connection, &selection.data_interface, None);
        return Err(error);
    }
    if let Err(error) = claim_interface(env, &connection, &selection.data_interface) {
        clear_pending_exception(env);
        close_open_connection(
            env,
            &connection,
            &selection.data_interface,
            selection.control_interface.as_ref(),
        );
        return Err(error);
    }
    let configure = if expected.vendor_id == SILABS_VENDOR_ID {
        configure_cp210x(env, &connection, selection.data_interface_id)
    } else {
        configure_cdc_acm(env, &connection, selection.control_interface_id)
    };
    if let Err(error) = configure {
        clear_pending_exception(env);
        close_open_connection(
            env,
            &connection,
            &selection.data_interface,
            selection.control_interface.as_ref(),
        );
        return Err(error);
    }

    let native = (|| {
        Ok(NativeUsbPort {
            vm,
            connection: env.new_global_ref(&connection)?,
            data_interface: env.new_global_ref(&selection.data_interface)?,
            control_interface: selection
                .control_interface
                .as_ref()
                .map(|interface| env.new_global_ref(interface))
                .transpose()?,
            read_endpoint: env.new_global_ref(&selection.read_endpoint)?,
            write_endpoint: env.new_global_ref(&selection.write_endpoint)?,
        })
    })();
    if native.is_err() {
        clear_pending_exception(env);
        close_open_connection(
            env,
            &connection,
            &selection.data_interface,
            selection.control_interface.as_ref(),
        );
    }
    native
}

fn clear_pending_exception(env: &mut JNIEnv<'_>) {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
}

fn close_open_connection(
    env: &mut JNIEnv<'_>,
    connection: &JObject<'_>,
    data_interface: &JObject<'_>,
    control_interface: Option<&JObject<'_>>,
) {
    let _ = env.call_method(
        connection,
        "releaseInterface",
        "(Landroid/hardware/usb/UsbInterface;)Z",
        &[JValue::Object(data_interface)],
    );
    if let Some(control) = control_interface {
        let _ = env.call_method(
            connection,
            "releaseInterface",
            "(Landroid/hardware/usb/UsbInterface;)Z",
            &[JValue::Object(control)],
        );
    }
    let _ = env.call_method(connection, "close", "()V", &[]);
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
}

struct NativePortSelection<'local> {
    data_interface: JObject<'local>,
    control_interface: Option<JObject<'local>>,
    read_endpoint: JObject<'local>,
    write_endpoint: JObject<'local>,
    data_interface_id: i32,
    control_interface_id: i32,
}

fn select_native_port<'local>(
    env: &mut JNIEnv<'local>,
    device: &JObject<'_>,
    vendor_id: i32,
) -> jni::errors::Result<NativePortSelection<'local>> {
    let count = env.call_method(device, "getInterfaceCount", "()I", &[])?.i()?;
    let mut control = None;
    let mut data = None;
    for index in 0..count {
        let interface = env
            .call_method(
                device,
                "getInterface",
                "(I)Landroid/hardware/usb/UsbInterface;",
                &[JValue::Int(index)],
            )?
            .l()?;
        let class = env.call_method(&interface, "getInterfaceClass", "()I", &[])?.i()?;
        let id = env.call_method(&interface, "getId", "()I", &[])?.i()?;
        if class == USB_CLASS_COMM {
            control = Some((interface, id));
            continue;
        }
        if class != USB_CLASS_CDC_DATA
            && !(vendor_id == SILABS_VENDOR_ID && class == USB_CLASS_VENDOR_SPEC)
        {
            continue;
        }
        if let Some((read, write)) = bulk_endpoints(env, &interface)? {
            data = Some((interface, id, read, write));
            break;
        }
    }
    let Some((data_interface, data_interface_id, read_endpoint, write_endpoint)) = data else {
        return Err(invalid_arguments());
    };
    if vendor_id != SILABS_VENDOR_ID && control.is_none() {
        return Err(invalid_arguments());
    }
    let (control_interface, control_interface_id) =
        control.map_or((None, data_interface_id), |(interface, id)| (Some(interface), id));
    Ok(NativePortSelection {
        data_interface,
        control_interface,
        read_endpoint,
        write_endpoint,
        data_interface_id,
        control_interface_id,
    })
}

fn bulk_endpoints<'local>(
    env: &mut JNIEnv<'local>,
    interface: &JObject<'_>,
) -> jni::errors::Result<Option<(JObject<'local>, JObject<'local>)>> {
    let count = env.call_method(interface, "getEndpointCount", "()I", &[])?.i()?;
    let mut read = None;
    let mut write = None;
    for index in 0..count {
        let endpoint = env
            .call_method(
                interface,
                "getEndpoint",
                "(I)Landroid/hardware/usb/UsbEndpoint;",
                &[JValue::Int(index)],
            )?
            .l()?;
        if env.call_method(&endpoint, "getType", "()I", &[])?.i()? != USB_ENDPOINT_XFER_BULK {
            continue;
        }
        if env.call_method(&endpoint, "getDirection", "()I", &[])?.i()? == USB_DIR_IN {
            read = Some(endpoint);
        } else {
            write = Some(endpoint);
        }
    }
    Ok(read.zip(write))
}

fn claim_interface(
    env: &mut JNIEnv<'_>,
    connection: &JObject<'_>,
    interface: &JObject<'_>,
) -> jni::errors::Result<()> {
    let claimed = env
        .call_method(
            connection,
            "claimInterface",
            "(Landroid/hardware/usb/UsbInterface;Z)Z",
            &[JValue::Object(interface), JValue::Bool(1)],
        )?
        .z()?;
    if claimed { Ok(()) } else { Err(invalid_arguments()) }
}

fn configure_cp210x(
    env: &mut JNIEnv<'_>,
    connection: &JObject<'_>,
    interface_id: i32,
) -> jni::errors::Result<()> {
    control_transfer(env, connection, 0x41, 0x00, 0x0001, interface_id, &[])?;
    control_transfer(env, connection, 0x41, 0x1e, 0, interface_id, &115_200_u32.to_le_bytes())?;
    control_transfer(env, connection, 0x41, 0x03, 0x0800, interface_id, &[])?;
    control_transfer(env, connection, 0x41, 0x07, 0x0303, interface_id, &[])
}

fn configure_cdc_acm(
    env: &mut JNIEnv<'_>,
    connection: &JObject<'_>,
    interface_id: i32,
) -> jni::errors::Result<()> {
    let mut line = Vec::from(115_200_u32.to_le_bytes());
    line.extend([0, 0, 8]);
    control_transfer(env, connection, 0x21, 0x20, 0, interface_id, &line)?;
    control_transfer(env, connection, 0x21, 0x22, 0x0003, interface_id, &[])
}

fn control_transfer(
    env: &mut JNIEnv<'_>,
    connection: &JObject<'_>,
    request_type: i32,
    request: i32,
    value: i32,
    index: i32,
    data: &[u8],
) -> jni::errors::Result<()> {
    let array = env.byte_array_from_slice(data)?;
    let transferred = env
        .call_method(
            connection,
            "controlTransfer",
            "(IIII[BII)I",
            &[
                JValue::Int(request_type),
                JValue::Int(request),
                JValue::Int(value),
                JValue::Int(index),
                JValue::Object(&array),
                JValue::Int(jni_len(data.len())?),
                JValue::Int(USB_WRITE_TIMEOUT_MS),
            ],
        )?
        .i()?;
    if transferred == jni_len(data.len())? { Ok(()) } else { Err(invalid_arguments()) }
}

fn usb_manager<'local>(
    env: &mut JNIEnv<'local>,
    activity: &JObject<'_>,
) -> jni::errors::Result<JObject<'local>> {
    let service = env.new_string("usb")?;
    env.call_method(
        activity,
        "getSystemService",
        "(Ljava/lang/String;)Ljava/lang/Object;",
        &[JValue::Object(&service)],
    )?
    .l()
}

fn find_device<'local>(
    env: &mut JNIEnv<'local>,
    manager: &JObject<'_>,
    expected: &AndroidUsbAttachment,
) -> jni::errors::Result<Option<JObject<'local>>> {
    let devices = env.call_method(manager, "getDeviceList", "()Ljava/util/HashMap;", &[])?.l()?;
    let values = env.call_method(&devices, "values", "()Ljava/util/Collection;", &[])?.l()?;
    let iterator = env.call_method(&values, "iterator", "()Ljava/util/Iterator;", &[])?.l()?;
    while env.call_method(&iterator, "hasNext", "()Z", &[])?.z()? {
        let device = env.call_method(&iterator, "next", "()Ljava/lang/Object;", &[])?.l()?;
        if attachment(env, &device)? == *expected {
            return Ok(Some(device));
        }
    }
    Ok(None)
}

fn attachment(
    env: &mut JNIEnv<'_>,
    device: &JObject<'_>,
) -> jni::errors::Result<AndroidUsbAttachment> {
    let name = env.call_method(device, "getDeviceName", "()Ljava/lang/String;", &[])?.l()?;
    Ok(AndroidUsbAttachment {
        device_id: env.call_method(device, "getDeviceId", "()I", &[])?.i()?,
        vendor_id: env.call_method(device, "getVendorId", "()I", &[])?.i()?,
        product_id: env.call_method(device, "getProductId", "()I", &[])?.i()?,
        device_name: env.get_string(&JString::from(name))?.to_string_lossy().into_owned(),
    })
}

async fn dispatch_query<T, F>(query: F) -> Result<T, PlatformFailure>
where
    T: Send + 'static,
    F: FnOnce(&mut JNIEnv<'_>, &JObject<'_>) -> jni::errors::Result<T> + Send + 'static,
{
    let (sender, receiver) = async_channel::bounded(1);
    wry::try_dispatch(move |env, activity, _| {
        let result = if activity.is_null() {
            Err(failure("android_activity_unavailable", true))
        } else {
            query(env, activity)
                .map_err(|_| clear_jni_failure(env, "android_usb_open_failed", true))
        };
        let _ = sender.try_send(result);
    })
    .map_err(|_| failure("android_dispatch_unavailable", true))?;
    tokio::time::timeout(std::time::Duration::from_secs(5), receiver.recv())
        .await
        .map_err(|_| failure("android_dispatch_timeout", true))?
        .map_err(|_| failure("android_dispatch_closed", true))?
}

fn clear_jni_failure(env: &mut JNIEnv<'_>, code: &str, retryable: bool) -> PlatformFailure {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    failure(code, retryable)
}

fn invalid_arguments() -> jni::errors::Error {
    jni::errors::Error::JniCall(jni::errors::JniError::InvalidArguments)
}

fn jni_len(value: usize) -> jni::errors::Result<i32> {
    i32::try_from(value).map_err(|_| invalid_arguments())
}

fn failure(code: &str, retryable: bool) -> PlatformFailure {
    PlatformFailure { code: code.into(), retryable }
}
