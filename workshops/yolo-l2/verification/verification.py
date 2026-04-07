import numpy as np
from flask import Flask, request, jsonify
from PIL import Image
from ultralytics import YOLO
import os

app = Flask(__name__)

# --- CONFIG ---
FLAG = os.environ.get("FLAG", "flag{ChristianSzegedy_Is_Proud}")
SOURCE_IMAGE_PATH = os.environ.get("SOURCE_IMAGE_PATH", "nika.png")
L2_THRESHOLD = float(os.environ.get("L2_THRESHOLD", 50.0))

# OPTIONAL: Set this to force a targeted attack (e.g. make the model see a toaster)
# If not set, the default behavior is just to "break" the classification (untargeted).
raw_target = os.environ.get("TARGET_CLASS")
TARGET_CLASS = raw_target if raw_target is not None else None

print("Loading Model and Reference Image...")
model = YOLO("yolo11n-cls.pt") 

def top_class(results):
    r = results[0]
    top_idx = int(r.probs.top1)
    conf = float(r.probs.top1conf)
    class_name = r.names[top_idx]
    return (class_name, conf)

try:
    ref_img = Image.open(SOURCE_IMAGE_PATH).convert("RGB")
    REF_IMG_ARRAY = np.array(ref_img, dtype=np.float32)
    REF_SHAPE = REF_IMG_ARRAY.shape
    print(f"Reference Loaded: {REF_SHAPE}")
    results = model(ref_img)
    r = results[0]
    ORIGINAL_CLASS_ID = int(r.probs.top1)
    ORIGINAL_CLASS =class_name = r.names[ORIGINAL_CLASS_ID]
    if TARGET_CLASS is not None:
        print(f"⚠️  MODE: Targeted Attack -> Must reach Class ID {TARGET_CLASS}")
    else:
        print(f"⚠️  MODE: Untargeted Attack -> Must escape Class ID {ORIGINAL_CLASS_ID}")

except Exception as e:
    print(f"CRITICAL ERROR: Could not load source image {SOURCE_IMAGE_PATH}: {e}")
    exit(1)



print("Ready.")

@app.route('/verify', methods=['POST'])
def verify():
    if 'image' not in request.files:
        return jsonify({"success": False, "message": "No image file provided"}), 400
    
    file = request.files['image']
    
    try:
        # 1. Load User Image
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
        l2_distance = float(np.linalg.norm(diff)) 

        if l2_distance > L2_THRESHOLD:
            return jsonify({
                "success": False, 
                "message": f"L2 Distance too high: {l2_distance:.2f} > {L2_THRESHOLD}"
            }), 400

        # 4. Check Classification
        results = model(user_img, verbose=False) 
        
        # Extract top prediction
        r = results[0]
        if hasattr(r, 'probs') and r.probs is not None:
            top_class_id = int(r.probs.top1)
            class_name = r.names[top_class_id]
            top_conf = float(r.probs.top1conf)
        else:
            return jsonify({"success": False, "message": "Server Error: Model is not a classifier."}), 500
        
        # 5. Determine Win Condition
        attack_success = False
        fail_message = ""

        if TARGET_CLASS is not None:
            # Targeted Attack Logic
            if top_class_id == TARGET_CLASS:
                attack_success = True
            else:
                fail_message = f"Target Class {TARGET_CLASS} not reached. Got Class {class_name}."
        else:
            # Untargeted Attack Logic (Default)
            if top_class_id != ORIGINAL_CLASS_ID:
                attack_success = True
            else:
                fail_message = f"Object (Class {ORIGINAL_CLASS}) still detected."

        # 6. Return Response
        if attack_success:
            return jsonify({
                "success": True,
                "message": "Adversarial Attack Successful!",
                "l2_distance": l2_distance,
                "flag": FLAG,
                "predicted_class": class_name,
                "confidence": top_conf
            })
        else:
            return jsonify({
                "success": False,
                "message": fail_message,
                "l2_distance": l2_distance,
                "predicted_class": class_name,
                "confidence": top_conf
            }), 200

    except Exception as e:
        return jsonify({"success": False, "message": f"Server Error: {str(e)}"}), 500

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5000)