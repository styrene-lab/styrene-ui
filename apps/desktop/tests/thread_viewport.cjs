const {test} = require('node:test');
const assert = require('node:assert/strict');
const {scrollIntent, timestampLabel, shouldMarkRead} = require('../src/assets/thread_viewport.js');

test('opening a populated thread anchors to the newest message', () => {
  assert.equal(scrollIntent(null, {id:'a', outgoing:false}, false), 'latest');
});
test('a new outgoing message anchors even while reading history', () => {
  assert.equal(scrollIntent('a', {id:'b', outgoing:true}, false), 'latest');
});
test('incoming messages follow only when already at the bottom', () => {
  assert.equal(scrollIntent('a', {id:'b', outgoing:false}, true), 'latest');
  assert.equal(scrollIntent('a', {id:'b', outgoing:false}, false), 'preserve');
});
test('receipts, earlier history and opening details do not jump a reader', () => {
  assert.equal(scrollIntent('b', {id:'b', outgoing:true}, false), 'preserve');
});
test('clearing the thread does not invent a scroll destination', () => {
  assert.equal(scrollIntent('b', null, false), 'preserve');
});
test('recency changes at the minute and hour boundaries', () => {
  const now = Date.UTC(2026,8,7,12)/1000;
  assert.equal(timestampLabel(now-59,now,'en-US'), 'Just now');
  assert.equal(timestampLabel(now-60,now,'en-US'), '1m ago');
  assert.equal(timestampLabel(now-900,now,'en-US'), '15m ago');
  assert.equal(timestampLabel(now-3599,now,'en-US'), '59m ago');
  assert.match(timestampLabel(now-3600,now,'en-US'), /Sep/);
  assert.match(timestampLabel(now-86400,now,'en-US'), /Sep/);
  assert.equal(timestampLabel(0,now), 'Time unavailable');
  assert.equal(timestampLabel(NaN,now), 'Time unavailable');
  assert.doesNotMatch(timestampLabel(now+3600,now,'en-US'), /ago/);
});

test('only a focused visible latest incoming message is marked read', () => {
  const context = {pinned:true, focused:true, visible:true, modal:false, incomingId:'m1', reportedId:null};
  assert.equal(shouldMarkRead(context), true);
  for (const change of [{pinned:false},{focused:false},{visible:false},{modal:true},{incomingId:null},{reportedId:'m1'}]) {
    assert.equal(shouldMarkRead({...context,...change}), false);
  }
  assert.equal(shouldMarkRead({...context,incomingId:'m2',reportedId:'m1'}), true);
});
