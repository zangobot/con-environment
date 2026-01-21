import numpy as np
from flask import Flask, request, jsonify
from PIL import Image
from ultralytics import YOLO
import os

app = Flask(__name__)

# --- CONFIG ---
FLAG = os.environ.get("FLAG", "ChristianSzegedy")
SOURCE_IMAGE_PATH = os.environ.get("SOURCE_IMAGE_PATH", "nika.png")
L2_THRESHOLD = os.environ.get("L2_THRESHOLD", 50.0 )
ORIGINAL_CLASS_ID = os.environ.get("ORIGINAL_CLASS_ID", 0)

# Load resources once on startup
print("Loading Model and Reference Image...")
model = YOLO("yolo11n-cls.pt") 
ref_img = Image.open(SOURCE_IMAGE_PATH).convert("RGB")
REF_IMG_ARRAY = np.array(ref_img, dtype=np.float32)
REF_SHAPE = REF_IMG_ARRAY.shape
print("Ready.")

@app.route('/verify', methods=['POST'])
def verify():
    # 1. Receive Image via API
    if 'image' not in request.files:
        return jsonify({"success": False, "message": "No image file provided"}), 400
    
    file = request.files['image']
    
    try:
        # Load user image directly from memory
        user_img = Image.open(file.stream).convert("RGB")
        user_img_array = np.array(user_img, dtype=np.float32)

        # 2. Check Dimensions
        if user_img_array.shape != REF_SHAPE:
            return jsonify({
                "success": False, 
                "message": f"Dimension mismatch. Expected {REF_SHAPE}, got {user_img_array.shape}"
            }), 400

        # 3. Check L2 Constraint
        diff = user_img_array - REF_IMG_ARRAY
        l2_distance = float(np.linalg.norm(diff)) # Convert to float for JSON serialization

        if l2_distance > L2_THRESHOLD:
            return jsonify({
                "success": False, 
                "message": f"L2 Distance too high: {l2_distance:.2f} > {L2_THRESHOLD}"
            }), 400

        # 4. Check Classification
        results = model(user_img, verbose=False) 
        
        detected_original = False
        detected_classes = []
        
        # Check all detections
        for r in results:
            for box in r.boxes:
                cls_id = int(box.cls[0])
                conf = float(box.conf[0])
                detected_classes.append(cls_id)
                if cls_id == ORIGINAL_CLASS_ID:
                    detected_original = True
        
        # 5. Result Logic
        if not detected_original:
            # Success! Read flag
            with open(FLAG_PATH, "r") as f:
                flag = f.read().strip()
            return jsonify({
                "success": True,
                "message": "Adversarial Attack Successful!",
                "l2_distance": l2_distance,
                "flag": flag
            })
        else:
            return jsonify({
                "success": False,
                "message": f"Object (Class {ORIGINAL_CLASS_ID}) still detected.",
                "l2_distance": l2_distance,
                "detected_classes": detected_classes
            }), 200

    except Exception as e:
        return jsonify({"success": False, "message": str(e)}), 500

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5000)