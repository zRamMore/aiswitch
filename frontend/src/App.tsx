import { ProviderList } from "@/components/provider-list";
import { Button } from "@/components/ui/button";
import { PlusCircle } from "lucide-react";
import {
  useActiveProviderQuery,
  useDeleteProviderMutation,
  useProvidersQuery,
  useSetActiveProviderMutation,
} from "@/api";
import { Dialog, DialogTrigger } from "./components/ui/dialog";
import { useState } from "react";
import { ProviderEditDialog } from "./components/provider-edit";

export default function ProviderManager() {
  const [open, setOpen] = useState(false);
  const {
    data: providersData,
    isLoading: providersIsLoading,
    isError: providersIsError,
  } = useProvidersQuery();
  const {
    data: activeProvider,
    isLoading: activeProviderIsLoading,
    isError: activeProviderIsError,
  } = useActiveProviderQuery();
  const [setActiveProvider] = useSetActiveProviderMutation();
  const [deleteProviderMutation] = useDeleteProviderMutation();

  const deleteProvider = (id: string) => {
    deleteProviderMutation(id);
  };

  const setActive = (provider: string | null) => {
    setActiveProvider(provider);
  };

  if (providersIsLoading || activeProviderIsLoading) {
    return <div>Loading...</div>;
  }

  if (providersIsError || activeProviderIsError) {
    return <div>Error!</div>;
  }

  const providers = providersData!;

  return (
    <div>
      <h1 className="text-2xl font-bold mb-4">Provider Manager</h1>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogTrigger asChild>
          <Button className="mb-4">
            <PlusCircle className="mr-2 h-4 w-4" /> Add New Provider
          </Button>
        </DialogTrigger>
        <ProviderEditDialog
          onClose={() => setOpen(false)}
          newProvider
          providersIds={providers.map((p) => p.id)}
          provider={{
            id: "",
            name: "",
            api_url: "",
            api_key: "",
            presets: [],
          }}
        />
      </Dialog>
      <ProviderList
        providers={providers}
        activeProvider={activeProvider!}
        onDelete={deleteProvider}
        onSetActive={setActive}
      />
    </div>
  );
}
