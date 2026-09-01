// Copyright 2020-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

package {{package}}

import android.annotation.SuppressLint
import android.app.PendingIntent
import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.res.Configuration
import android.hardware.usb.UsbDevice
import android.hardware.usb.UsbManager
import android.os.Build
import android.os.Bundle
import android.webkit.WebView
import android.view.KeyEvent
import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.ProcessLifecycleOwner

private val ACTIVITY_ID_KEY = "__wryActivityId"
private const val IDENTITY_DOCUMENT_REQUEST = 0x53544944

object WryLifecycleObserver : DefaultLifecycleObserver {
    override fun onCreate(owner: LifecycleOwner) {
        super.onCreate(owner)
        Rust.create()
        Rust.wryCreate()
    }

    override fun onStart(owner: LifecycleOwner) {
        super.onStart(owner)
        Rust.start()
    }

    override fun onResume(owner: LifecycleOwner) {
        super.onResume(owner)
        Rust.resume()
    }

    override fun onPause(owner: LifecycleOwner) {
        super.onPause(owner)
        Rust.pause()
    }

    override fun onStop(owner: LifecycleOwner) {
        super.onStop(owner)
        Rust.stop()
    }
}

abstract class WryActivity : AppCompatActivity() {
    private lateinit var mWebView: RustWebView
    private var usbPermissionReceiver: BroadcastReceiver? = null
    private var usbPermissionPendingIntent: PendingIntent? = null
    private var identityBackupSharePending = false
    private var identityBackupSharePaused = false
    var id: Int = 0
    open val handleBackNavigation: Boolean = true

    open fun onWebViewCreate(webView: WebView) { }

    fun setWebView(webView: RustWebView) {
        mWebView = webView

        if (handleBackNavigation) {
            val callback = object : OnBackPressedCallback(true) {
                override fun handleOnBackPressed() {
                    if (this@WryActivity::mWebView.isInitialized) {
                        if (this@WryActivity.mWebView.canGoBack()) {
                            this@WryActivity.mWebView.goBack()
                        } else {
                            this.isEnabled = false
                            this@WryActivity.onBackPressed()
                            this.isEnabled = true
                        }
                    }
                }
            }
            onBackPressedDispatcher.addCallback(this, callback)
        }

        onWebViewCreate(webView)
    }

    val version: String
        @SuppressLint("WebViewApiAvailability", "ObsoleteSdkInt")
        get() {
            // Check getCurrentWebViewPackage() directly if above Android 8
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                return WebView.getCurrentWebViewPackage()?.versionName ?: ""
            }

            // Otherwise manually check WebView versions
            var webViewPackage = "com.google.android.webview"
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
              webViewPackage = "com.android.chrome"
            }
            try {
                @Suppress("DEPRECATION")
                val info = packageManager.getPackageInfo(webViewPackage, 0)
                return info.versionName.toString()
            } catch (ex: Exception) {
                Logger.warn("Unable to get package info for '$webViewPackage'$ex")
            }

            try {
                @Suppress("DEPRECATION")
                val info = packageManager.getPackageInfo("com.android.webview", 0)
                return info.versionName.toString()
            } catch (ex: Exception) {
                Logger.warn("Unable to get package info for 'com.android.webview'$ex")
            }

            // Could not detect any webview, return empty string
            return ""
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        id = savedInstanceState?.getInt(ACTIVITY_ID_KEY) ?: intent.extras?.getInt(ACTIVITY_ID_KEY) ?: hashCode()
        ProcessLifecycleOwner.get().lifecycle.addObserver(WryLifecycleObserver)
        removeIdentityBackupShare()
        Rust.onActivityCreate(this)
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        Rust.onWindowFocusChanged(this, hasFocus)
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        outState.putInt(ACTIVITY_ID_KEY, id)
        Rust.onActivitySaveInstanceState()
    }

    override fun onPause() {
        super.onPause()
        if (identityBackupSharePending) {
            identityBackupSharePaused = true
        }
        if (::mWebView.isInitialized) {
            mWebView.onPause()
        }
    }

    override fun onResume() {
        super.onResume()
        if (identityBackupSharePending && identityBackupSharePaused) {
            removeIdentityBackupShare()
        }
        if (::mWebView.isInitialized) {
            mWebView.onResume()
        }
    }

    override fun onDestroy() {
        cancelUsbPermissionRequest()
        removeIdentityBackupShare()
        super.onDestroy()
        Rust.onActivityDestroy(this)
        Rust.onWebviewDestroy(this, if (::mWebView.isInitialized) { mWebView.id } else { "" })
    }

    override fun onLowMemory() {
        super.onLowMemory()
        Rust.onActivityLowMemory()
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        Rust.onConfigurationChanged()
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        Rust.onNewIntent(intent)
    }

    fun requestIdentityBackupDocument(): Boolean {
        return try {
            val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "application/octet-stream"
            }
            @Suppress("DEPRECATION")
            startActivityForResult(intent, IDENTITY_DOCUMENT_REQUEST)
            true
        } catch (_: RuntimeException) {
            false
        }
    }

    fun presentIdentityBackup(document: ByteArray): Int {
        if (document.size > IdentityBackupProvider.MAX_DOCUMENT_BYTES || identityBackupSharePending) {
            return SHARE_PRESENTATION_FAILED
        }
        val sendIntent = Intent(Intent.ACTION_SEND).apply {
            type = IdentityBackupProvider.MIME_TYPE
        }
        if (sendIntent.resolveActivity(packageManager) == null) {
            return SHARE_UNAVAILABLE
        }
        removeIdentityBackupShare()
        val file = IdentityBackupProvider.shareFile(this)
        return try {
            if (!file.parentFile!!.mkdirs() && !file.parentFile!!.isDirectory) {
                return SHARE_PRESENTATION_FAILED
            }
            file.outputStream().use { stream -> stream.write(document) }
            val uri = IdentityBackupProvider.shareUri(this)
            sendIntent.apply {
                putExtra(Intent.EXTRA_STREAM, uri)
                clipData = ClipData.newRawUri("Encrypted identity backup", uri)
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }
            identityBackupSharePending = true
            identityBackupSharePaused = false
            startActivity(Intent.createChooser(sendIntent, null))
            SHARE_PRESENTED
        } catch (_: ActivityNotFoundException) {
            removeIdentityBackupShare()
            SHARE_UNAVAILABLE
        } catch (_: RuntimeException) {
            removeIdentityBackupShare()
            SHARE_PRESENTATION_FAILED
        } catch (_: java.io.IOException) {
            removeIdentityBackupShare()
            SHARE_PRESENTATION_FAILED
        }
    }

    private fun removeIdentityBackupShare() {
        revokeUriPermission(
            IdentityBackupProvider.shareUri(this),
            Intent.FLAG_GRANT_READ_URI_PERMISSION
        )
        IdentityBackupProvider.shareFile(this).delete()
        identityBackupSharePending = false
        identityBackupSharePaused = false
    }

    @Deprecated("Deprecated in Android")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != IDENTITY_DOCUMENT_REQUEST) {
            return
        }
        val uri = data?.data
        if (resultCode == RESULT_OK && uri != null) {
            Rust.onDocumentPickerResult(uri.toString(), false)
        } else {
            Rust.onDocumentPickerResult("", true)
        }
    }

    fun requestUsbPermission(device: UsbDevice, action: String): Boolean {
        if (usbPermissionReceiver != null) {
            return false
        }
        val expectedName = device.deviceName
        val receiver = object : BroadcastReceiver() {
            override fun onReceive(context: Context, intent: Intent) {
                if (intent.action != action) {
                    return
                }
                val resultDevice = if (Build.VERSION.SDK_INT >= 33) {
                    intent.getParcelableExtra(UsbManager.EXTRA_DEVICE, UsbDevice::class.java)
                } else {
                    @Suppress("DEPRECATION")
                    intent.getParcelableExtra(UsbManager.EXTRA_DEVICE)
                }
                if (resultDevice?.deviceName != expectedName) {
                    return
                }
                val granted = intent.getBooleanExtra(UsbManager.EXTRA_PERMISSION_GRANTED, false)
                cancelUsbPermissionRequest()
                Rust.onUsbPermissionResult(expectedName, granted)
            }
        }
        usbPermissionReceiver = receiver
        try {
            val filter = IntentFilter(action)
            if (Build.VERSION.SDK_INT >= 33) {
                registerReceiver(receiver, filter, Context.RECEIVER_NOT_EXPORTED)
            } else {
                registerReceiver(receiver, filter)
            }
            val intent = Intent(action).setPackage(packageName)
            val flags = PendingIntent.FLAG_ONE_SHOT or PendingIntent.FLAG_CANCEL_CURRENT or
                if (Build.VERSION.SDK_INT >= 31) PendingIntent.FLAG_MUTABLE else 0
            val pendingIntent = PendingIntent.getBroadcast(this, 0, intent, flags)
            usbPermissionPendingIntent = pendingIntent
            val manager = getSystemService(Context.USB_SERVICE) as? UsbManager
                ?: throw IllegalStateException("USB service unavailable")
            manager.requestPermission(device, pendingIntent)
            return true
        } catch (_: RuntimeException) {
            cancelUsbPermissionRequest()
            return false
        }
    }

    fun cancelUsbPermissionRequest() {
        usbPermissionPendingIntent?.cancel()
        usbPermissionPendingIntent = null
        val receiver = usbPermissionReceiver ?: return
        usbPermissionReceiver = null
        try {
            unregisterReceiver(receiver)
        } catch (_: IllegalArgumentException) {
            // Registration failed or the activity was already tearing down.
        }
    }

    fun getAppClass(name: String): Class<*> {
        return Class.forName(name)
    }

    fun startActivity(cls: Class<*>): Int {
        val intent = Intent(this, cls)
        val id = kotlin.random.Random.nextInt()
        intent.putExtra(ACTIVITY_ID_KEY, id)
        startActivity(intent)
        return id
    }

    {{class-extension}}
}

private const val SHARE_PRESENTED = 0
private const val SHARE_UNAVAILABLE = 1
private const val SHARE_PRESENTATION_FAILED = 2
