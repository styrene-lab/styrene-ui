// The renderer owns geometry and local date formatting, never delivery decisions.
function scrollIntent(previousLastId, last, pinned) {
  if (!last) return 'preserve';
  if (!previousLastId || pinned || (last.id !== previousLastId && last.outgoing)) return 'latest';
  return 'preserve';
}
function timestampLabel(timestamp, now = Date.now()/1000, locale) {
  if (!Number.isFinite(timestamp) || timestamp <= 0) return 'Time unavailable';
  const age = now - timestamp;
  if (age >= 0 && age < 60) return 'Just now';
  if (age >= 60 && age < 3600) return `${Math.floor(age/60)}m ago`;
  const date = new Date(timestamp*1000);
  if (!Number.isFinite(date.getTime())) return 'Time unavailable';
  return new Intl.DateTimeFormat(locale, {
    month:'short', day:'numeric', year: date.getFullYear() === new Date(now*1000).getFullYear() ? undefined : 'numeric',
    hour:'numeric', minute:'2-digit'
  }).format(date);
}
function shouldMarkRead({pinned, focused, visible, modal, incomingId, reportedId}) {
  return pinned && focused && visible && !modal && Boolean(incomingId) && incomingId !== reportedId;
}
if (typeof module !== 'undefined' && module.exports) {
  module.exports = {scrollIntent, timestampLabel, shouldMarkRead};
} else { return mountViewport(); }
async function mountViewport() {
  const viewport = document.getElementById('chat-viewport');
  if (viewport && !viewport.dataset.scrollBound) {
    viewport.dataset.scrollBound = 'true';
    const stream = viewport.querySelector('.message-stream');
    const jump = viewport.parentElement.querySelector('.chat-jump-latest');
    let pinned = true, previousLastId = null, anchor = null, anchorTop = 0, frame = 0;
    let reportedId = null;
    const reportVisible = () => {
      const incoming = [...stream.querySelectorAll('.message-item[data-outgoing="false"]')].at(-1);
      const incomingId = incoming?.dataset.messageId;
      if (typeof dioxus !== 'undefined' && shouldMarkRead({pinned, focused:document.hasFocus(), visible:document.visibilityState === 'visible', modal:Boolean(document.querySelector('dialog[open]')), incomingId, reportedId})) {
        dioxus.send(incomingId);
        reportedId = incomingId;
      }
    };
    const onFocus = () => { reportedId = null; reportVisible(); };
    window.addEventListener('focus', onFocus);
    document.addEventListener('visibilitychange', onFocus);
    const messages = () => [...stream.querySelectorAll('.message-item')];
    const remember = () => {
      const top = viewport.getBoundingClientRect().top;
      anchor = messages().find(node => node.getBoundingClientRect().bottom > top);
      anchorTop = anchor ? anchor.getBoundingClientRect().top : 0;
    };
    const latest = () => {
      viewport.scrollTop = viewport.scrollHeight;
      pinned = true;
      if (jump) jump.hidden = true;
      remember();
    };
    const updateTimes = () => {
      for (const time of stream.querySelectorAll('time[data-message-time]')) {
        const timestamp = Number(time.dataset.messageTime);
        const label = timestampLabel(timestamp);
        if (time.textContent !== label) time.textContent = label;
        const date = new Date(timestamp*1000);
        if (timestamp > 0 && Number.isFinite(date.getTime())) {
          time.dateTime = date.toISOString();
          time.title = date.toLocaleString();
        }
      }
    };
    const reconcile = () => {
      frame = 0;
      if (!viewport.isConnected) return;
      const lastNode = messages().at(-1);
      const last = lastNode ? {id:lastNode.dataset.messageId, outgoing:lastNode.dataset.outgoing === 'true'} : null;
      if (scrollIntent(previousLastId, last, pinned) === 'latest') latest();
      else {
        if (anchor?.isConnected) viewport.scrollTop += anchor.getBoundingClientRect().top - anchorTop;
        if (jump && last?.id !== previousLastId) jump.hidden = false;
        remember();
      }
      previousLastId = last?.id ?? null;
      updateTimes();
      reportVisible();
    };
    const schedule = () => { if (!frame) frame = requestAnimationFrame(reconcile); };
    const onScroll = () => {
      pinned = viewport.scrollHeight - viewport.clientHeight - viewport.scrollTop < 48;
      if (pinned && jump) jump.hidden = true;
      remember();
      reportVisible();
    };
    viewport.addEventListener('scroll', onScroll, {passive:true});
    jump?.addEventListener('click', latest);
    const changes = new MutationObserver(records => {
      if (records.some(record => !(record.target.parentElement?.closest('time') || record.target.closest?.('time')))) schedule();
    });
    changes.observe(stream, {childList:true, subtree:true, characterData:true});
    const resize = new ResizeObserver(schedule);
    resize.observe(stream);
    resize.observe(viewport);
    const timer = setInterval(updateTimes, 30000);
    let release;
    const lifetime = new Promise(resolve => { release = resolve; });
    const removal = new MutationObserver(() => {
      if (!viewport.isConnected) {
        changes.disconnect(); resize.disconnect(); removal.disconnect();
        clearInterval(timer); cancelAnimationFrame(frame);
        viewport.removeEventListener('scroll', onScroll);
        jump?.removeEventListener('click', latest);
        window.removeEventListener('focus', onFocus);
        document.removeEventListener('visibilitychange', onFocus);
        release();
      }
    });
    removal.observe(document.body, {childList:true, subtree:true});
    schedule();
    await lifetime;
  }
  return null;
}
