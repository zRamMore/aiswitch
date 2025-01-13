import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { Provider } from "./types/provider";

export interface LogOverview {
  id: string;
  provider_id: string;
  model: string;
  request_tokens?: number;
  request_time: string;
  response_tokens?: number;
  response_time: string;
  chat: boolean;
}

export interface ChatMessage {
  role: "system" | "assistant" | "user";
  content: string;
}

export interface CompletionRequest {
  model: string;
  prompt: string;
  stream: boolean;
}

export interface ChatRequest {
  model: string;
  messages: ChatMessage[];
  stream: boolean;
}

export interface ChatResponse {
  id: string;
  created: number;
  model: string;
  choices: { index: number; message: ChatMessage }[];
}

export interface ChatResponseChunk {
  choices: { delta: { content: string } }[];
}

export interface CompletionResponse {
  id: string;
  created: number;
  model: string;
  choices: { index: number; text: string }[];
}

interface LogEntry extends LogOverview {
  request: ChatRequest | CompletionRequest;
  response?:
    | ChatResponse
    | ChatResponseChunk[]
    | CompletionResponse
    | CompletionResponse[];
}

export const api = createApi({
  baseQuery: fetchBaseQuery({ baseUrl: "/api" }),
  tagTypes: ["Providers", "ActiveProvider"],
  endpoints: (builder) => ({
    providers: builder.query<Provider[], void>({
      query: () => "config/providers",
      providesTags: ["Providers"],
    }),
    activeProvider: builder.query<string, void>({
      query: () => "config/active-provider",
      providesTags: ["ActiveProvider"],
    }),
    addProvider: builder.mutation<void, { id: string; properties: Provider }>({
      query: (provider) => ({
        url: `config/providers/${provider.id}`,
        method: "POST",
        body: provider.properties,
      }),
      invalidatesTags: ["Providers"],
    }),
    updateProvider: builder.mutation<
      void,
      { id: string; properties: Partial<Provider> }
    >({
      query: (provider) => ({
        url: `config/providers/${provider.id}`,
        method: "PUT",
        body: provider.properties,
      }),
      invalidatesTags: ["Providers"],
    }),
    deleteProvider: builder.mutation<void, string>({
      query: (id) => ({
        url: `config/providers/${id}`,
        method: "DELETE",
      }),
      invalidatesTags: ["Providers"],
    }),
    setActiveProvider: builder.mutation<void, string | null>({
      query: (provider) => ({
        url: `config/active-provider`,
        method: "POST",
        body: provider ?? "",
      }),
      invalidatesTags: ["ActiveProvider"],
    }),
    getLogs: builder.query<
      { logs: LogOverview[]; rowCount: number },
      {
        pageIndex: number;
        pageSize: number;
        sorting?: {
          by: string;
          desc: boolean;
        };
      }
    >({
      query: ({ pageIndex, pageSize, sorting }) => ({
        url: `logs`,
        params: {
          page: pageIndex,
          size: pageSize,
          sort: sorting
            ? `${sorting.by},${sorting.desc ? "desc" : "asc"}`
            : undefined,
        },
      }),
    }),
    getLog: builder.query<LogEntry, string>({
      query: (id) => `logs/${id}`,
    }),
  }),
});

export const {
  useProvidersQuery,
  useActiveProviderQuery,
  useAddProviderMutation,
  useUpdateProviderMutation,
  useDeleteProviderMutation,
  useSetActiveProviderMutation,
  useGetLogsQuery,
  useGetLogQuery,
} = api;
