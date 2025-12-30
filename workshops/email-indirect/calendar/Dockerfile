FROM python:3.9-slim

RUN pip install flask

WORKDIR /app
COPY service.py .

EXPOSE 6000

CMD ["python", "service.py"]