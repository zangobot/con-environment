import threading
import smtplib
from email.message import EmailMessage
from flask import Flask, render_template_string, request, redirect, url_for
from aiosmtpd.controller import Controller
import asyncio

# --- CONFIGURATION ---
CHALLENGE_SERVER_HOST = 'ctf-email-service' # K8s DNS name of the server
CHALLENGE_SERVER_PORT = 25
MY_LISTEN_PORT = 25  # Must capture replies on port 25
WEB_PORT = 5000

# --- GLOBAL STORE ---
# In a real app, use a DB. For a CTF container, memory is fine.
inbox = []

# --- 1. SMTP LISTENER (Background Thread) ---
class ReplyHandler:
    async def handle_DATA(self, server, session, envelope):
        peer = session.peer
        mail_from = envelope.mail_from
        data = envelope.content.decode('utf8', errors='replace')
        
        print(f"[Client] Received reply from {peer}")
        
        # Parse simplified content for the UI
        inbox.insert(0, {
            'from': mail_from,
            'body': data
        })
        return '250 OK'

def run_smtp_listener():
    # We create a new event loop for this thread because aiosmtpd requires it
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    
    handler = ReplyHandler()
    controller = Controller(handler, hostname='0.0.0.0', port=MY_LISTEN_PORT)
    controller.start()
    print(f"[*] Client SMTP Listener active on port {MY_LISTEN_PORT}")
    
    try:
        loop.run_forever()
    except KeyboardInterrupt:
        controller.stop()

# --- 2. WEB FRONTEND (Flask) ---
app = Flask(__name__)

# Simple HTML Template embedded for single-file portability
HTML_TEMPLATE = """
<!DOCTYPE html>
<html>
<head>
    <title>CTF Email Client</title>
    <style>
        body { font-family: monospace; background: #222; color: #0f0; max-width: 800px; margin: 2rem auto; }
        .box { border: 1px solid #444; padding: 1rem; margin-bottom: 1rem; background: #111; }
        input, textarea { width: 100%; background: #000; color: #fff; border: 1px solid #555; padding: 5px; }
        button { background: #0f0; color: #000; border: none; padding: 10px 20px; cursor: pointer; font-weight: bold; }
        hr { border-color: #444; }
        .email-item { border-bottom: 1px dashed #555; padding: 10px 0; }
        .meta { color: #888; font-size: 0.9em; }
    </style>
</head>
<body>
    <h1>✉️ Secure Comms Terminal</h1>
    
    <div class="box">
        <h3>Compose Message</h3>
        <form action="/send" method="POST">
            <label>To:</label>
            <input type="text" value="challenge-bot@ctf.local" disabled>
            <br><br>
            <label>Subject:</label>
            <input type="text" name="subject" required>
            <br><br>
            <label>Body:</label>
            <textarea name="body" rows="4" required></textarea>
            <br><br>
            <button type="submit">SEND TRANSMISSION</button>
        </form>
    </div>

    <div class="box">
        <h3>Inbox (Auto-refreshes)</h3>
        <p class="meta">Listening for callbacks on Port 25...</p>
        <button onclick="window.location.reload()">Refresh Inbox</button>
        <hr>
        {% for email in emails %}
            <div class="email-item">
                <div class="meta">From: {{ email.from }}</div>
                <pre>{{ email.body }}</pre>
            </div>
        {% else %}
            <p><i>No messages received yet.</i></p>
        {% endfor %}
    </div>
</body>
</html>
"""

@app.route('/')
def index():
    return render_template_string(HTML_TEMPLATE, emails=inbox)

@app.route('/send', methods=['POST'])
def send_email():
    subject = request.form.get('subject')
    body = request.form.get('body')
    
    msg = EmailMessage()
    msg.set_content(body)
    msg['Subject'] = subject
    msg['From'] = "contestant@client-pod" # The server uses this to determine who to reply to
    msg['To'] = "challenge-bot@ctf.local"

    try:
        # Connect to the Challenge Service
        with smtplib.SMTP(CHALLENGE_SERVER_HOST, CHALLENGE_SERVER_PORT) as s:
            s.send_message(msg)
        print("[Client] Email sent successfully.")
    except Exception as e:
        print(f"[Client] Error sending email: {e}")
        return f"Error sending email: {e}", 500

    return redirect(url_for('index'))

if __name__ == '__main__':
    # Start SMTP Listener in background thread
    t = threading.Thread(target=run_smtp_listener, daemon=True)
    t.start()
    
    # Start Web Server
    app.run(host='0.0.0.0', port=WEB_PORT)