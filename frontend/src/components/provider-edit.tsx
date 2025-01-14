import {
  DialogClose,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import { Button } from "./ui/button";
import { InputKey } from "./InputKey";
import { Label } from "./ui/label";
import { Input } from "./ui/input";
import { Provider } from "@/types/provider";
import { useState } from "react";
import { useAddProviderMutation, useUpdateProviderMutation } from "@/api";

interface ProviderEditDialogProps {
  provider: Provider;
  newProvider?: boolean;
  onClose: () => void;
}

export const ProviderEditDialog = ({
  provider: initialData,
  newProvider,
  onClose,
}: ProviderEditDialogProps) => {
  const [provider, setProvider] = useState(initialData);
  const [addProviderMutation] = useAddProviderMutation();
  const [updateProviderMutation] = useUpdateProviderMutation();

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (newProvider) {
      addProviderMutation({
        id: provider.id,
        properties: {
          name: provider.name,
          id: provider.id,
          api_url: provider.api_url,
          api_key: provider.api_key,
          presets: [],
        },
      });
    } else {
      updateProviderMutation({
        id: provider.id,
        properties: {
          name: provider.name,
          api_url: provider.api_url,
          api_key: provider.api_key,
        },
      });
    }
    onClose();
  };

  return (
    <DialogContent>
      <form onSubmit={handleSubmit}>
        <DialogHeader>
          <DialogTitle>{newProvider ? "Create" : "Edit"} Provider</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 mb-4">
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
              onChange={(e) => setProvider({ ...provider, id: e.target.value })}
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
        </div>
        <DialogFooter>
          <DialogClose asChild>
            <Button type="button" variant="outline">
              Cancel
            </Button>
          </DialogClose>
          <Button type="submit">
            {newProvider ? "Create" : "Update"} Provider
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  );
};
