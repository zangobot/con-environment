FROM python:3.11-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    libgl1 \
    libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/*

# 1. Install CPU-only PyTorch first
RUN pip install --no-cache-dir  torch==2.9.1 torchvision==0.24.1 torchaudio==2.9.1 --index-url https://download.pytorch.org/whl/cpu
ENV MPLBACKEND=Agg
# 2. Install dependencies (Headless OpenCV + No-Deps Ultralytics)
# We strictly manage dependencies to keep the image small and CPU-bound.
RUN pip install --no-cache-dir \
    ultralytics-opencv-headless \
    flask \
    numpy \
    pillow

# Download YOLO model during build
RUN python -c "from ultralytics import YOLO; YOLO('yolo11n-cls.pt')"

# Copy challenge files
COPY verification.py .
COPY nika.png .

# Create a non-root user for security
RUN useradd -m ctf && \
    mkdir -p /tmp/Ultralytics && \
    chown -R ctf:ctf /tmp/Ultralytics

USER ctf

EXPOSE 5000

CMD ["python", "verification.py"]