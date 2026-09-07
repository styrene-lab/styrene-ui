// Run on macOS: swift apps/desktop/tests/viewport_webkit.swift
// Uses the production viewport controller and stylesheet in real WebKit.
import AppKit
import WebKit

let app = NSApplication.shared
let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
let web = WKWebView(frame: NSRect(x: 0, y: 0, width: 700, height: 500))
let window = NSWindow(contentRect: web.frame, styleMask: [.titled], backing: .buffered, defer: false)
window.title = "Styrene thread viewport verification"
window.contentView = web

final class Runner: NSObject, WKNavigationDelegate {
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        webView.callAsyncJavaScript(#"""
            const wait = () => new Promise(resolve => setTimeout(resolve, 150));
            const viewport = document.getElementById('chat-viewport');
            const stream = document.querySelector('.message-stream');
            const make = (id, outgoing=false) => {
                const node = document.createElement('article');
                node.className = 'message-item'; node.dataset.messageId = String(id); node.dataset.outgoing = String(outgoing);
                node.innerHTML = `<div class="message received"><div class="message-content">Fixture message ${id}</div></div><footer class="message-caption"><time data-message-time="1788777600">…</time></footer>`;
                return node;
            };
            const bottom = () => viewport.scrollHeight - viewport.clientHeight - viewport.scrollTop < 2;
            const check = (condition, reason) => { if (!condition) throw Error(reason); };
            for (let i=1; i<=40; i++) stream.append(make(i));
            await wait(); check(bottom(), 'initial history did not anchor');
            viewport.scrollTop = 200;
            await wait(); const position = viewport.scrollTop;
            stream.append(make(41));
            await wait(); check(Math.abs(viewport.scrollTop-position)<2, 'incoming message stole reading position');
            check(!document.querySelector('.chat-jump-latest').hidden, 'new message affordance missing');
            const anchor = stream.children[5], before = anchor.getBoundingClientRect().top;
            stream.prepend(make(0, true));
            await wait(); check(Math.abs(anchor.getBoundingClientRect().top-before)<2, 'prepended history moved visible content');
            stream.append(make(42, true));
            await wait(); check(bottom(), 'outgoing message did not anchor');
            stream.lastElementChild.querySelector('.message-content').textContent = 'Attachment grew\n'.repeat(12);
            await wait(); check(bottom(), 'late content growth detached the anchor');
            viewport.scrollTop=100; await wait();
            document.querySelector('.chat-jump-latest').click();
            await wait(); check(bottom(), 'jump to latest did not anchor');
            const date = stream.querySelector('time');
            check(date.dateTime.length > 0 && date.textContent !== '…', 'timestamp not formatted');
            return 'PASS: initial anchor, incoming preservation, prepend preservation, outgoing anchor, content resize, jump to latest, timestamp';
        """#, arguments: [:], in: nil, in: .page) { result in
            switch result {
            case .success(let value): print(value); exit(0)
            case .failure(let error): print("FAIL: \(error)"); exit(1)
            }
        }
    }
}
let runner = Runner()
web.navigationDelegate = runner
let css = try String(contentsOf: root.appendingPathComponent("src/assets/style.css"), encoding: .utf8)
let script = try String(contentsOf: root.appendingPathComponent("src/assets/thread_viewport.js"), encoding: .utf8)
let html = """
<!doctype html><html><head><style>\(css)
:root { --surface:#eee; --text:#111; --border:#888; --text-muted:#444; --radius-control:0; }
body { height:500px; } .peer-detail { width:650px; height:450px; }
</style></head><body><div class="peer-detail"><button hidden class="chat-jump-latest">Latest</button>
<div class="chat-area" id="chat-viewport"><div class="message-stream"></div></div></div>
<script>\(script)</script></body></html>
"""
web.loadHTMLString(html, baseURL: nil)
window.orderFront(nil)
DispatchQueue.main.asyncAfter(deadline: .now() + 20) { print("FAIL: WebKit timeout"); exit(1) }
app.run()
