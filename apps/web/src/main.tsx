import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter } from "react-router";
import "./index.css";
import App from "./App.tsx";
import { Toaster } from "@/components/ui/sonner";
import { ApiError } from "@/api/client";
import { AuthProvider } from "@/auth/auth";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 15_000,
      retry: (failureCount, error) =>
        error instanceof ApiError && error.status >= 400 && error.status < 500
          ? false
          : failureCount < 1,
      refetchOnWindowFocus: false,
    },
  },
});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <BrowserRouter>
      <AuthProvider>
        <QueryClientProvider client={queryClient}>
          <App />
          <Toaster richColors position="bottom-right" />
        </QueryClientProvider>
      </AuthProvider>
    </BrowserRouter>
  </StrictMode>,
);
