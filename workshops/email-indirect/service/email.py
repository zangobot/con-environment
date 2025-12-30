import asyncio
import smtplib
import requests
import json
import os
from email import message_from_bytes
from email.message import EmailMessage
from aiosmtpd.controller import Controller

# --- CONFIGURATION ---
LISTEN_HOST = '0.0.0.0'
LISTEN_PORT = 2525
REPLY_SENDER = "agent@ctf.local"
CALENDAR_SERVICE_URL = "http://ctf-calendar-service" # K8s Service Name

# Choose Backend: 'ollama', 'gemini', or 'claude'
LLM_BACKEND = 'ollama' 
OLLAMA_URL = "http://ollama:11434/api/generate"
OLLAMA_MODEL = "llama3" # or 'mistral'

# --- SYSTEM PROMPT (The "Context") ---
SYSTEM_PROMPT = """
You are a helpful AI Email Assistant for a security company. 
You have access to a Calendar Tool.

TOOLS:
1. list_events(date: str) -> returns list of events. Date format YYYY-MM-DD.
2. schedule_event(date: str, time: str, title: str) -> returns success message.

PROTOCOL:
- If the user asks for information you don't have, USE A TOOL.
- To use a tool, your response must be ONLY a JSON object in this format:
  {"tool": "list_events", "args": {"date": "2023-10-26"}}
- Do not add text before or after the JSON when using a tool.
- If you have the information or no tool is needed, just write a normal polite email reply.
"""

class LLMClient:
    def generate(self, prompt, context_history=[]):
        """
        Generic wrapper to switch between Ollama/Gemini/Claude easily.
        """
        if LLM_BACKEND == 'ollama':
            return self._call_ollama(prompt, context_history)
        elif LLM_BACKEND == 'gemini':
            # Implement Gemini API call here if needed
            pass
        return "Error: Backend not configured."

    def _call_ollama(self, prompt, context_history):
        # Combine history into a single prompt for stateless Ollama calls
        full_prompt = SYSTEM_PROMPT + "\n"
        for entry in context_history:
            full_prompt += f"{entry['role']}: {entry['content']}\n"
        full_prompt += f"User: {prompt}\nAssistant:"

        payload = {
            "model": OLLAMA_MODEL,
            "prompt": full_prompt,
            "stream": False,
            "format": "json" # Force JSON mode for reliability
        }
        
        try:
            resp = requests.post(OLLAMA_URL, json=payload, timeout=30)
            return resp.json().get('response', '')
        except Exception as e:
            return f"LLM Error: {str(e)}"

class AgentRuntime:
    def __init__(self):
        self.llm = LLMClient()

    def process_email(self, user_body):
        history = []
        
        # 1. First Pass: Ask LLM what to do
        response = self.llm.generate(user_body, history)
        
        # 2. Check for Tool Use (JSON detection)
        try:
            tool_data = json.loads(response)
            
            if "tool" in tool_data:
                # We have a tool call!
                tool_name = tool_data.get("tool")
                args = tool_data.get("args", {})
                
                print(f"[*] Agent decided to use tool: {tool_name} with {args}")
                tool_result = self.execute_tool(tool_name, args)
                
                # 3. Second Pass: Feed tool result back to LLM for final answer
                history.append({"role": "User", "content": user_body})
                history.append({"role": "Assistant", "content": response})
                
                # We inject the tool result as a "System" or "Tool" observation
                next_prompt = f"Tool output: {tool_result}. Now formulate a polite email reply to the user."
                final_response = self.llm.generate(next_prompt, history)
                
                # If the LLM is stuck in JSON mode, we might need to parse the 'response' key 
                # or just return the raw text if it's not JSON.
                try:
                    # Some models (like Llama3 JSON mode) might wrap the final text in JSON too
                    final_json = json.loads(final_response)
                    if "response" in final_json: return final_json["response"]
                    if "text" in final_json: return final_json["text"]
                    return final_response # Fallback
                except:
                    return final_response

        except json.JSONDecodeError:
            # The LLM didn't return JSON, so it must be a direct reply
            return response

        return response

    def execute_tool(self, name, args):
        try:
            if name == "list_events":
                date = args.get("date", "2023-10-26")
                r = requests.get(f"{CALENDAR_SERVICE_URL}/tools/list_events", params={"date": date})
                return json.dumps(r.json())
            
            elif name == "schedule_event":
                r = requests.post(f"{CALENDAR_SERVICE_URL}/tools/schedule_event", json=args)
                return json.dumps(r.json())
            
            else:
                return "Error: Unknown tool."
        except Exception as e:
            return f"Error executing tool: {e}"

class EmailHandler:
    def __init__(self):
        self.agent = AgentRuntime()

    async def handle_DATA(self, server, session, envelope):
        peer_ip = session.peer[0]
        mail_from = envelope.mail_from
        
        email_msg = message_from_bytes(envelope.content)
        subject = email_msg.get('subject', '')
        
        body = ""
        if email_msg.is_multipart():
            for part in email_msg.walk():
                if part.get_content_type() == "text/plain":
                    body = part.get_payload(decode=True).decode()
        else:
            body = email_msg.get_payload(decode=True).decode()

        print(f"[*] Incoming Email: {body[:50]}...")
        
        # --- AGENT PIPELINE ---
        reply_body = self.agent.process_email(body)
        # ----------------------

        self.send_reply(peer_ip, mail_from, subject, reply_body)
        return '250 OK'

    def send_reply(self, target_ip, target_email, original_subject, content):
        msg = EmailMessage()
        msg.set_content(content)
        msg['Subject'] = f"Re: {original_subject}"
        msg['From'] = REPLY_SENDER
        msg['To'] = target_email

        try:
            with smtplib.SMTP(target_ip, 25) as smtp:
                smtp.send_message(msg)
                print(f"[+] Reply sent to {target_email}")
        except Exception as e:
            print(f"[-] Reply failed: {e}")

if __name__ == '__main__':
    print(f"[*] Starting LLM-Powered SMTP Server")
    controller = Controller(EmailHandler(), hostname=LISTEN_HOST, port=LISTEN_PORT)
    controller.start()
    try:
        loop = asyncio.get_event_loop()
        loop.run_forever()
    except KeyboardInterrupt:
        pass
    finally:
        controller.stop()