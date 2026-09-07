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
if (typeof module !== 'undefined' && module.exports) {
  module.exports = {scrollIntent, timestampLabel};
} else {
  const viewport = document.getElementById('chat-viewport');
  if (viewport && !viewport.dataset.scrollBound) {
    viewport.dataset.scrollBound = 'true';
    const stream = viewport.querySelector('.message-stream');
    const jump = viewport.parentElement.querySelector('.chat-jump-latest');
    let pinned = true, previousLastId = null, anchor = null, anchorTop = 0, frame = 0;
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
    };
    const schedule = () => { if (!frame) frame = requestAnimationFrame(reconcile); };
    const onScroll = () => {
      pinned = viewport.scrollHeight - viewport.clientHeight - viewport.scrollTop < 48;
      if (pinned && jump) jump.hidden = true;
      remember();
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
    const removal = new MutationObserver(() => {
      if (!viewport.isConnected) {
        changes.disconnect(); resize.disconnect(); removal.disconnect();
        clearInterval(timer); cancelAnimationFrame(frame);
        viewport.removeEventListener('scroll', onScroll);
        jump?.removeEventListener('click', latest);
      }
    });
    removal.observe(document.body, {childList:true, subtree:true});
    schedule();
  }
}
