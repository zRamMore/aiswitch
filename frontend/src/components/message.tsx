import { useEffect, useRef, useState } from "react";
import { MemoizedMarkdown } from "./memoized-markdown";
import { Button } from "./ui/button";
import { Pencil, PencilOff } from "lucide-react";
import { Textarea } from "./ui/textarea";

interface MessageProps {
  id: string;
  sender: string;
  content: string;
}

export const Message = ({ id, sender, content }: MessageProps) => {
  const [renderContent, setRenderContent] = useState(true);

  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [textareaHeight, setTextareaHeight] = useState("auto");

  useEffect(() => {
    if (textareaRef.current) {
      setTextareaHeight(`${textareaRef.current.scrollHeight}px`);
    }
  }, [content, renderContent]);

  return (
    <div>
      <div className="flex">
        <div className="font-bold mb-2">{sender}</div>
        <Button
          variant={"ghost"}
          onClick={() => setRenderContent(!renderContent)}
          className="ml-auto w-6 h-6 p-4"
        >
          {renderContent ? <Pencil /> : <PencilOff />}
        </Button>
      </div>
      <div className="prose space-y-2">
        {renderContent ? (
          <MemoizedMarkdown id={id} content={content} />
        ) : (
          <Textarea
            value={content}
            readOnly
            className="overflow-hidden"
            style={{ height: textareaHeight, resize: "none" }}
            ref={textareaRef}
          />
        )}
      </div>
    </div>
  );
};
