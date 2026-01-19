FROM python:3.12-slim

WORKDIR /workspace

# Install Python libraries
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy the notebooks inside the container
COPY deep-networks.ipynb /workspace/deep-networks.ipynb
COPY domain-knowledge.ipynb /workspace/domain-knowledge.ipynb

RUN git clone https://github.com/ytisf/theZoo.git
ENV ZIP_PASSWORD=infected
ENV ZOO_DIR=/workspace/theZoo/malware/Binaries
ENV MALWARE_SAMPLES=/workspace/malware_samples

RUN mkdir $MALWARE_SAMPLES

RUN mkdir -p $MALWARE_SAMPLES
RUN find $ZOO_DIR -type f -name "*.zip" -exec unzip -o -P "infected" {} -d $MALWARE_SAMPLES \;
RUN rm -f $MALWARE_SAMPLES/*.zip
RUN find $MALWARE_SAMPLES -type f -exec sh -c "
    h=\$(sha256sum \"{}\" | cut -d\" \" -f1);
    mv \"{}\" \"$MALWARE_SAMPLES/\$h.\file\"
    " \;

# Create a non-root user (Standard practice for Jupyter)
RUN useradd -m player
RUN chown -R player:player /workspace
USER player

EXPOSE 8888

# Launch Jupyter Notebook (No token/password for local convenience)
CMD ["jupyter", "notebook", "--ip=0.0.0.0", "--port=8888", "--no-browser", "--NotebookApp.token=''", "--NotebookApp.password=''"]