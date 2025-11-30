#!/usr/bin/env python3
import http.server
import socketserver
import json
import subprocess

PORT = 8000

class WasmHandler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        '.wasm': 'application/wasm',
        '.js': 'application/javascript',
        '.mjs': 'application/javascript',
    }

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

                # Execute in Windows PowerShell non-interactive mode
                proc = subprocess.run([
                    'powershell.exe', '-NoProfile', '-NonInteractive', '-Command', cmd
                ], capture_output=True, text=True)

                resp = {
                    'stdout': proc.stdout,
                    'stderr': proc.stderr,
                    'returncode': proc.returncode,
                }
                self.send_response(200)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps(resp).encode('utf-8'))
            except Exception as e:
                self.send_response(500)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps({'error': str(e)}).encode('utf-8'))
        else:
            self.send_response(404)
            self.end_headers()

if __name__ == "__main__":
    socketserver.TCPServer.allow_reuse_address = True
    with socketserver.TCPServer(("", PORT), WasmHandler) as httpd:
        print(f"Serving at http://localhost:{PORT}")
        httpd.serve_forever()
