FROM python:3.9-slim

# Install system dependencies for OpenCV/GL
RUN apt-get update && apt-get install -y \
    libgl1 \
    libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Install Python dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Download YOLOv8 nano model during build so we don't download it on every container start
RUN python -c "from ultralytics import YOLO; YOLO('yolo11n-cls.pt')"

# Copy challenge files
COPY verification.py .
COPY nika.png .

# Create a non-root user for security
RUN useradd -m ctf
USER ctf

EXPOSE 5000

CMD ["python", "verification.py"]