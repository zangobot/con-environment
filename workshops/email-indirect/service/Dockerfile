FROM python:3.9-slim

# Install aiosmtpd
RUN pip install aiosmtpd

# Create a non-root user for security (optional, but good practice)
WORKDIR /app
COPY email.py .

# Expose the port we defined in the script
EXPOSE 2525

# Run unbuffered to see logs in K8s immediately
CMD ["python", "-u", "email.py"]