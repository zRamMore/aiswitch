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
  const presets = provider.presets;
  const [addPresetMutation] = useAddPresetMutation();
  const [updatePresetMutation] = useUpdatePresetMutation();

  const [selectedPreset, setSelectedPreset] = useState<string | undefined>(
    provider.preset
  );

  const [name, setName] = useState<string>(provider.name);
  const [id, setId] = useState<string>("");
  const [overrides, setOverrides] = useState<[string, string | number][]>([]);

  const updateToSelectedPreset = useCallback(() => {
    const preset = presets.find((p) => p.id === selectedPreset);
    if (preset) {
      setName(preset.name);
      setId(preset.id);
      setOverrides(Object.entries(preset.overrides));
    }
  }, [presets, selectedPreset]);

  useEffect(() => {
    updateToSelectedPreset();
  }, [updateToSelectedPreset]);

  const newPreset = (name: string) => {
    const id = name.toLowerCase().replace(/\s/g, "-");

    setOverrides([]);
    setName(name);
    setId(id);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (new Set(overrides.map((p) => p[0])).size !== overrides.length) {
      return;
    }
    if (overrides.some((p) => !p[0].length)) {
      return;
    }
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
            {provider.id &&
              provider.preset !== id &&
              provider.presets.find((p) => p.id === id) && (
                <p className="text-red-500">ID already exists</p>
              )}
            {!id && <p className="text-red-500">ID is required</p>}
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
            <div key={index} className="flex flex-col">
              <div className="flex items-center space-x-2">
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
              {(!prop[0].length ||
                overrides.filter((p) => p[0] === prop[0]).length > 1) && (
                <p className="text-red-500 bg-red-100 p-2 text-center my-2 rounded-md">
                  {overrides.filter((p) => p[0] === prop[0]).length > 1
                    ? "Property already exists"
                    : "Property name cannot be empty"}
                </p>
              )}
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
