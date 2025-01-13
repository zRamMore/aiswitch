export interface Preset {
  name: string;
  id: string;
  overrides: { [key: string]: string | number };
}

export interface Provider {
  id: string;
  name: string;
  api_url: string;
  api_key: string;
  presets: Preset[];
  preset?: string;
}
