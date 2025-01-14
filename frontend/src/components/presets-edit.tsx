import {
  DialogClose,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import { Button } from "./ui/button";
import { Provider } from "@/types/provider";
import { Combobox } from "./combobox";
import { useCallback, useEffect, useState } from "react";
import { Label } from "./ui/label";
import { Input } from "./ui/input";
import { PlusCircle, X } from "lucide-react";
import { useAddPresetMutation, useUpdatePresetMutation } from "@/api";
interface PresetEditDialogProps {
  provider: Provider;
  onClose: () => void;
}

export const PresetsEditorDialog = ({
  provider,
  onClose,
}: PresetEditDialogProps) => {
  const [presets, setPresets] = useState(provider.presets);
  const [addPresetMutation] = useAddPresetMutation();
  const [updatePresetMutation] = useUpdatePresetMutation();

  const [selectedPreset, setSelectedPreset] = useState<string | undefined>(
    provider.preset
  );

  const [name, setName] = useState<string>("");
  const [id, setId] = useState<string>("");
  const [overrides, setOverrides] = useState<[string, string | number][]>([]);

  const updateToSelectedPreset = useCallback(() => {
    const preset = presets.find((p) => p.id === selectedPreset);
    if (preset) {
      setId(preset.id);
      setOverrides(Object.entries(preset.overrides));
    }
  }, [presets, selectedPreset]);

  useEffect(() => {
    updateToSelectedPreset();
  }, [updateToSelectedPreset]);

  const newPreset = (name: string) => {
    const id = name.toLowerCase().replace(/\s/g, "-");

    const newPreset = {
      id,
      name,
      overrides: {},
    };
    setPresets((prev) => [...prev, newPreset]);
    setOverrides([]);
    setName(name);
    setSelectedPreset(id);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const parsedOverrides = overrides.map(([key, value]) => [
      key,
      isNaN(Number(value)) ? value : Number(value),
    ]);

    if (provider.presets.find((p) => p.id === id)) {
      updatePresetMutation({
        providerId: provider.id,
        presetId: id,
        preset: {
          id,
          name,
          overrides: Object.fromEntries(parsedOverrides),
        },
      });
    } else {
      addPresetMutation({
        providerId: provider.id,
        preset: {
          id,
          name,
          overrides: Object.fromEntries(parsedOverrides),
        },
      });
    }

    onClose();
  };

  return (
    <DialogContent>
      <DialogHeader>
        <DialogTitle>Edit Presets</DialogTitle>
      </DialogHeader>
      <Combobox
        mode="single" //single or multiple
        options={presets.map((preset) => ({
          label: preset.name,
          value: preset.id,
        }))}
        selected={selectedPreset}
        placeholder="Write a preset name"
        onChange={(value) => {
          setSelectedPreset(value as string);
          updateToSelectedPreset();
        }}
        onCreate={newPreset}
      />
      <form onSubmit={handleSubmit}>
        <div className="space-y-4 mb-4">
          <div className="space-y-2">
            <Label htmlFor="id">ID</Label>
            <Input
              id="id"
              value={id}
              onChange={(e) => setId(e.target.value)}
              required
            />
          </div>
        </div>
        <div className="space-y-2 flex flex-col">
          <Label>Overrides</Label>
          {overrides.map((prop, index) => (
            <div key={index} className="flex items-center space-x-2">
              <Button
                type="button"
                variant="ghost"
                size="icon"
                onClick={() =>
                  setOverrides((prev) => prev.filter((_, i) => i !== index))
                }
              >
                <X className="h-4 w-4" />
              </Button>
              <div className="flex items-center space-x-2 flex-grow">
                <Input
                  placeholder="Property Name"
                  value={prop[0]}
                  onChange={(e) => {
                    const value = e.target.value;
                    setOverrides((prev) =>
                      prev.map((p, i) => (i === index ? [value, p[1]] : p))
                    );
                  }}
                  className="flex-grow"
                />
                <Input
                  placeholder="Value"
                  value={prop[1] as string}
                  onChange={(e) => {
                    const value = e.target.value;
                    setOverrides((prev) =>
                      prev.map((p, i) => (i === index ? [p[0], value] : p))
                    );
                  }}
                  className="flex-grow"
                />
              </div>
            </div>
          ))}
          <div>
            <Button
              type="button"
              variant="outline"
              onClick={() => setOverrides((prev) => [...prev, ["", ""]])}
            >
              <PlusCircle className="mr-2 h-4 w-4" /> Add Override
            </Button>
          </div>
        </div>
        <DialogFooter>
          <DialogClose asChild>
            <Button type="button" variant="outline">
              Cancel
            </Button>
          </DialogClose>
          <Button type="submit">Save</Button>
        </DialogFooter>
      </form>
    </DialogContent>
  );
};
