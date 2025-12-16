#!/usr/bin/env python3
import http.server
import socketserver
import json
import subprocess
from datetime import datetime

PORT = 8000

def log(level, msg):
    """Print timestamped log message to console."""
    ts = datetime.now().strftime('%H:%M:%S.%f')[:-3]
    colors = {'INFO': '\033[94m', 'WARN': '\033[93m', 'ERR': '\033[91m', 'REQ': '\033[92m'}
    reset = '\033[0m'
    color = colors.get(level, '')
    print(f"{color}[{ts}] [{level}]{reset} {msg}")

class WasmHandler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        '.wasm': 'application/wasm',
        '.js': 'application/javascript',
        '.mjs': 'application/javascript',
    }

    def log_message(self, format, *args):
        """Override to use custom logging."""
        log('REQ', f"{self.address_string()} - {format % args}")

    def log_error(self, format, *args):
        """Override to use custom logging."""
        log('ERR', f"{self.address_string()} - {format % args}")

    def end_headers(self):
        # Disable caching during development
        self.send_header('Cache-Control', 'no-cache, no-store, must-revalidate')
        self.send_header('Pragma', 'no-cache')
        self.send_header('Expires', '0')
        super().end_headers()

    def do_POST(self):
        if self.path == '/exec':
            try:
                length = int(self.headers.get('Content-Length', '0'))
                raw = self.rfile.read(length).decode('utf-8') if length > 0 else '{}'
                data = json.loads(raw)
                cmd = data.get('cmd', '')
                if not cmd:
                    self.send_response(400)
                    self.send_header('Content-Type', 'application/json')
                    self.end_headers()
                    self.wfile.write(json.dumps({'error': 'missing cmd'}).encode('utf-8'))
                    return

                log('INFO', f"POST /exec cmd={cmd[:50]}{'...' if len(cmd)>50 else ''}")
                # Execute in Windows PowerShell
                proc = subprocess.run([
                    'powershell.exe', '-NoProfile', '-NonInteractive', '-Command', cmd
                ], capture_output=True, text=True)

                resp = {
                    'stdout': proc.stdout,
                    'stderr': proc.stderr,
                    'returncode': proc.returncode,
                }
                log('INFO', f"POST /exec result: rc={proc.returncode} stdout={len(proc.stdout)}B stderr={len(proc.stderr)}B")
                self.send_response(200)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps(resp).encode('utf-8'))
            except Exception as e:
                log('ERR', f"POST /exec exception: {e}")
                self.send_response(500)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps({'error': str(e)}).encode('utf-8'))
        else:
            log('WARN', f"POST to unknown path: {self.path}")
            self.send_response(404)
            self.end_headers()

if __name__ == "__main__":
    socketserver.TCPServer.allow_reuse_address = True
    log('INFO', f"Starting server on port {PORT}")
    with socketserver.TCPServer(("", PORT), WasmHandler) as httpd:
        print(f"Serving at http://localhost:{PORT}")
        log('INFO', "Server ready, press Ctrl+C to stop")
        httpd.serve_forever()
