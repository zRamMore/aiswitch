import { ChatMessage } from "@/api";
import { Message } from "./message";

interface ChatProps {
  systemName?: string;
  userName?: string;
  botName?: string;
  messages: ChatMessage[];
}

export const Chat = ({
  messages,
  botName,
  userName,
  systemName,
}: ChatProps) => {
  const roleToName = (role: string) => {
    if (role === "system") {
      return systemName || "System";
    }
    if (role === "user") {
      return userName || "You";
    }
    if (role === "assistant") {
      return botName || "Bot";
    }
    return role;
  };

  return (
    <div className="flex flex-col w-full mx-auto stretch">
      <div className="space-y-8 mb-4">
        {messages.map((message, index) => (
          <Message
            key={index}
            id={index.toString()}
            sender={roleToName(message.role)}
            content={message.content}
          />
        ))}
      </div>
    </div>
  );
};
