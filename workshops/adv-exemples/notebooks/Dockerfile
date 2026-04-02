FROM python:3.12-slim

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1 \
    PIP_NO_CACHE_DIR=1 \
    ZIP_PASSWORD=infected \
    WORKSPACE=/workspace \
    ZOO_DIR=/workspace/theZoo/malware/Binaries \
    MALWARE_SAMPLES=/workspace/malware_samples

WORKDIR ${WORKSPACE}

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    gcc \
    git \
    unzip \
 && rm -rf /var/lib/apt/lists/*

COPY requirements.txt goodware_strings.txt ./

RUN python -m pip install --upgrade pip setuptools wheel && \
    python -m pip install -r requirements.txt

RUN git clone --depth=1 https://github.com/ytisf/theZoo.git && \
    mkdir -p "${MALWARE_SAMPLES}" && \
    find "${ZOO_DIR}" -type f -name "*.zip" -exec unzip -o -P "${ZIP_PASSWORD}" {} -d "${MALWARE_SAMPLES}" \; && \
    find "${MALWARE_SAMPLES}" -type f -name "*.zip" -delete && \
    find "${MALWARE_SAMPLES}" -type f -print0 | \
    while IFS= read -r -d '' f; do \
        h=$(sha256sum "$f" | cut -d' ' -f1); \
        mv "$f" "${MALWARE_SAMPLES}/$h.file"; \
    done

RUN useradd --create-home --shell /bin/bash player && \
    chown -R player:player "${WORKSPACE}"

USER player

EXPOSE 8888

CMD ["jupyter", "notebook", "--ip=0.0.0.0", "--port=8888", "--no-browser", "--ServerApp.root_dir=/workspace", "--ServerApp.token=", "--ServerApp.password="]