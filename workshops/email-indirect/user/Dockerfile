FROM python:3.9-slim

# Install dependencies
RUN pip install aiosmtpd flask

WORKDIR /app
COPY client.py .

# Expose Web Port (5000) and SMTP Port (25)
EXPOSE 5000
EXPOSE 25

# We run as root here because binding port 25 usually requires it.
# In a strict CTF environment, you might use capabilities, but for this root is easiest.
CMD ["python", "-u", "client.py"]