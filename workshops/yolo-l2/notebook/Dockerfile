FROM python:3.11-slim

WORKDIR /workspace

RUN apt-get update && apt-get install -y --no-install-recommends \
    libgl1 \
    libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/*

RUN pip install --no-cache-dir  torch==2.9.1 torchvision==0.24.1 torchaudio==2.9.1 --index-url https://download.pytorch.org/whl/cpu

# 2. Install dependencies
# - 'opencv-python-headless': Removes need for apt-get libgl1 (saves ~100MB+ and build time)
# - 'ultralytics': Installed with --no-deps to stop it from re-downloading GPU PyTorch.
# - We manually list the dependencies ultralytics usually pulls in (pandas, seaborn, etc.)
RUN pip install --no-cache-dir \
    ultralytics-opencv-headless \
    flask \
    numpy \
    pillow \
    jupyter \
    matplotlib \
    requests \
    scipy

# Pre-download the YOLO model so players don't wait
RUN python -c "from ultralytics import YOLO; YOLO('yolo11n-cls.pt')"

# Copy challenge assets
COPY nika.png .
COPY notebook.ipynb .

# Create a non-root user (Standard practice for Jupyter)
RUN useradd -m player && \
    chown -R player:player /workspace && \
    # FIX: Give player ownership of the Ultralytics cache created by root
    mkdir -p /tmp/Ultralytics && \
    chown -R player:player /tmp/Ultralytics

USER player

EXPOSE 5000

# Launch Jupyter Notebook
CMD ["sh", "-c", "exec jupyter notebook \
    --ip=0.0.0.0 \
    --port=5000 \
    --no-browser \
    --NotebookApp.token='' \
    --NotebookApp.password='' \
    --NotebookApp.base_url=\"$BASE_URL\""]