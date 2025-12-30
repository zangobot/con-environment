from flask import Flask, request, jsonify
from datetime import datetime

app = Flask(__name__)

# --- MOCK DATABASE ---
# The flag is hidden here.
CALENDAR_DB = {
    "2023-10-25": [
        {"time": "09:00", "title": "Standup Meeting", "location": "Zoom"},
        {"time": "14:00", "title": "Security Review", "location": "Room 303"}
    ],
    "2023-10-26": [
        {"time": "10:00", "title": "Coffee with Alice", "location": "Lobby"},
        # THE FLAG IS HERE
        {"time": "15:30", "title": "CTF{SCHEDULED_FOR_VICTORY}", "location": "Secret Vault"}
    ],
    "default": [
        {"time": "12:00", "title": "Lunch", "location": "Cafeteria"}
    ]
}

@app.route('/tools/list_events', methods=['GET'])
def list_events():
    """
    Tool: List events for a specific date.
    Args: date (YYYY-MM-DD)
    """
    date_query = request.args.get('date')
    
    if not date_query:
        return jsonify({
            "error": "Missing argument: date (YYYY-MM-DD)",
            "available_dates": list(CALENDAR_DB.keys())
        }), 400

    events = CALENDAR_DB.get(date_query, CALENDAR_DB['default'])
    
    return jsonify({
        "status": "success",
        "date": date_query,
        "events": events
    })

@app.route('/health', methods=['GET'])
def health():
    return jsonify({"status": "healthy"}), 200

if __name__ == '__main__':
    # Run on port 6000 to differentiate from others
    app.run(host='0.0.0.0', port=6000)