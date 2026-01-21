FROM python:3.11-slim

# Install minimal system dependencies for OpenCV
RUN apt-get update && apt-get install -y --no-install-recommends \
    libgl1 \
    libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

WORKDIR /workspace

# Install CPU-only PyTorch first (much smaller than GPU version)
RUN pip install --no-cache-dir \
    torch torchvision --index-url https://download.pytorch.org/whl/cpu

# Install other dependencies
RUN pip install --no-cache-dir \
    flask \
    numpy \
    pillow \
    ultralytics \
    jupyter \
    matplotlib \
    requests

# Pre-download the YOLO model so players don't wait
RUN python -c "from ultralytics import YOLO; YOLO('yolo11n-cls.pt')"

# Copy challenge assets
COPY nika.png .
COPY notebook.ipynb .

# Create a non-root user (Standard practice for Jupyter)
RUN useradd -m player
RUN chown -R player:player /workspace
USER player

EXPOSE 8888

# Launch Jupyter Notebook (No token/password for local convenience)
CMD ["jupyter", "notebook", "--ip=0.0.0.0", "--port=8888", "--no-browser", "--NotebookApp.token=''", "--NotebookApp.password=''"]