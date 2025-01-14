import { Provider } from "@/types/provider";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Edit, Power, PowerOff, Trash2, Wrench } from "lucide-react";
import { Combobox } from "./combobox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "./ui/dialog";
import { DialogClose } from "@radix-ui/react-dialog";
import { ProviderEditDialog } from "./provider-edit";
import { useState } from "react";
import { PresetsEditorDialog } from "./presets-edit";

interface ProviderListProps {
  providers: Provider[];
  activeProvider: string | null;
  onDelete: (id: string) => void;
  onSetActive: (provider: string | null) => void;
}

export const ProviderList = ({
  providers,
  activeProvider,
  onDelete,
  onSetActive,
}: ProviderListProps) => {
  const [open, setOpen] = useState<string[]>([]);

  const openDialog = (id: string) => {
    setOpen([...open, id]);
  };

  const closeDialog = (id: string) => {
    setOpen(open.filter((i: string) => i !== id));
  };

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {providers.map((provider) => (
        <Card
          key={provider.id}
          className={`cursor-pointer ${
            activeProvider === provider.id ? "border-primary" : ""
          }`}
        >
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">
              {provider.name}
            </CardTitle>
            <div>
              <Button
                variant="ghost"
                size="icon"
                onClick={(e) => {
                  e.stopPropagation();
                  onSetActive(
                    activeProvider === provider.id ? null : provider.id
                  );
                }}
              >
                {activeProvider === provider.id ? (
                  <PowerOff className="h-4 w-4" />
                ) : (
                  <Power className="h-4 w-4" />
                )}
              </Button>
              <Dialog
                open={open.includes(`provider${provider.id}`)}
                onOpenChange={(o) =>
                  o
                    ? openDialog(`provider${provider.id}`)
                    : closeDialog(`provider${provider.id}`)
                }
              >
                <DialogTrigger asChild>
                  <Button variant="ghost" size="icon">
                    <Edit className="h-4 w-4" />
                  </Button>
                </DialogTrigger>
                <ProviderEditDialog
                  onClose={() => closeDialog(`provider${provider.id}`)}
                  provider={provider}
                />
              </Dialog>
              <Dialog>
                <DialogTrigger asChild>
                  <Button variant="ghost" size="icon">
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </DialogTrigger>
                <DialogContent>
                  <DialogHeader>
                    <DialogTitle>
                      Are you sure you want to delete this provider?
                    </DialogTitle>
                    <DialogDescription>
                      This action cannot be undone.
                    </DialogDescription>
                  </DialogHeader>
                  <DialogFooter>
                    <DialogClose asChild>
                      <Button
                        onClick={(e) => {
                          e.stopPropagation();
                          onDelete(provider.id);
                        }}
                        variant="destructive"
                      >
                        Delete
                      </Button>
                    </DialogClose>
                    <DialogClose asChild>
                      <Button variant="ghost">Cancel</Button>
                    </DialogClose>
                  </DialogFooter>
                </DialogContent>
              </Dialog>
            </div>
          </CardHeader>
          <CardContent>
            <p className="text-xs text-muted-foreground">
              URL: {provider.api_url}
            </p>
            <div className="mt-2 text-xs text-muted-foreground flex gap-2">
              <Combobox
                mode="single" //single or multiple
                className="flex-1"
                options={provider.presets.map((preset) => ({
                  label: preset.name,
                  value: preset.id,
                }))}
                selected={provider.preset}
                placeholder="Select a preset..."
                onChange={(value) => console.log(value)}
                onCreate={() => {}}
              />
              <Dialog
                open={open.includes(`presets${provider.id}`)}
                onOpenChange={(o) =>
                  o
                    ? openDialog(`presets${provider.id}`)
                    : closeDialog(`presets${provider.id}`)
                }
              >
                <DialogTrigger asChild>
                  <Button variant="ghost" size="icon">
                    <Wrench className="h-4 w-4" />
                  </Button>
                </DialogTrigger>
                <PresetsEditorDialog
                  onClose={() => closeDialog(`presets${provider.id}`)}
                  provider={provider}
                />
              </Dialog>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
};
