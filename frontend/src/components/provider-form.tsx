import { useState } from "react";
import { Provider } from "@/types/provider";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { InputKey } from "./InputKey";

interface ProviderFormProps {
  onSubmit: (provider: Provider) => void;
  onCancel: () => void;
  initialProvider?: Provider | null;
}

export const ProviderForm = ({
  onSubmit,
  onCancel,
  initialProvider,
}: ProviderFormProps) => {
  const [provider, setProvider] = useState<Provider>(
    initialProvider || {
      id: "",
      name: "",
      api_url: "",
      api_key: "",
      presets: [],
    }
  );

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit(provider);
  };

  return (
    <div
      className="fixed inset-0 flex items-center justify-center bg-black bg-opacity-50 z-50"
      onClick={onCancel}
    >
      <Card
        className="w-full max-w-2xl mx-auto"
        onClick={(e) => e.stopPropagation()}
      >
        <CardHeader>
          <CardTitle>
            {initialProvider ? "Edit Provider" : "Add New Provider"}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="name">Name</Label>
              <Input
                id="name"
                value={provider.name}
                onChange={(e) =>
                  setProvider({ ...provider, name: e.target.value })
                }
                required
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="id">ID</Label>
              <Input
                id="id"
                value={provider.id}
                onChange={(e) =>
                  setProvider({ ...provider, id: e.target.value })
                }
                required
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="url">URL</Label>
              <Input
                id="url"
                value={provider.api_url}
                onChange={(e) =>
                  setProvider({ ...provider, api_url: e.target.value })
                }
                required
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="key">Api key</Label>
              <InputKey
                id="key"
                value={provider.api_key}
                onChange={(e) =>
                  setProvider({ ...provider, api_key: e.target.value })
                }
                required
              />
            </div>
            <div className="flex justify-end space-x-2">
              <Button type="button" variant="outline" onClick={onCancel}>
                Cancel
              </Button>
              <Button type="submit">
                {initialProvider ? "Update" : "Add"} Provider
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
};
