FROM python:3.9-slim

# Install system dependencies for OpenCV (required by YOLO)
RUN apt-get update && apt-get install -y \
    libgl1-mesa-glx \
    libglib2.0-0 \
    wget \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

# Install Python libraries
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Pre-download the YOLO model so players don't wait
RUN python -c "from ultralytics import YOLO; YOLO('yolov8n.pt')"

# Copy challenge assets
COPY source_image.jpg .
COPY solve_template.ipynb .

# Create a non-root user (Standard practice for Jupyter)
RUN useradd -m player
RUN chown -R player:player /workspace
USER player

EXPOSE 8888

# Launch Jupyter Notebook (No token/password for local convenience)
CMD ["jupyter", "notebook", "--ip=0.0.0.0", "--port=8888", "--no-browser", "--NotebookApp.token=''", "--NotebookApp.password=''"]