import { useParams } from "react-router";
import {
  ChatMessage,
  ChatRequest,
  ChatResponse,
  ChatResponseChunk,
  CompletionRequest,
  CompletionResponse,
  useGetLogQuery,
} from "./api";
import { Chat } from "./components/chat";

export default function Review() {
  const { id } = useParams<{ id: string }>();
  const { data, isLoading, isError } = useGetLogQuery(id as string);

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (isError) {
    return <div>Error!</div>;
  }

  const chat = data!.chat;
  const request = data!.request;
  let messages: ChatMessage[] = [];

  const streamingResponse = request.stream ?? false;
  const response = data!.response;

  if (chat) {
    messages = (request as ChatRequest).messages;

    if (response) {
      if (streamingResponse) {
        const responseChunks = response as ChatResponseChunk[];
        let message = "";
        responseChunks.forEach((chunk) => {
          message += chunk.choices[0]!.delta.content;
        });
        messages = [
          ...messages,
          {
            role: "assistant",
            content: message,
          },
        ];
      } else {
        const responseMessage = (response as ChatResponse).choices[0]!;
        messages = [...messages, responseMessage.message];
      }
    }
  } else {
    messages.push({
      role: "user",
      content: (request as CompletionRequest).prompt,
    });

    if (response) {
      if (streamingResponse) {
        const responseChunks = response as CompletionResponse[];
        let message = "";
        responseChunks.forEach((chunk) => {
          message += chunk.choices[0]!.text;
        });
        messages = [
          ...messages,
          {
            role: "assistant",
            content: message,
          },
        ];
      } else {
        const responseMessage = (response as CompletionResponse).choices[0]!
          .text;
        messages = [
          ...messages,
          {
            role: "assistant",
            content: responseMessage,
          },
        ];
      }
    }
  }

  return <Chat botName={request.model} messages={messages} />;
}
