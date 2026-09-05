import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router";
import "./index.css";
import App from "./App.tsx";
import { Toaster } from "@/components/ui/sonner";
import { AuthProvider } from "@/auth/auth";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <BrowserRouter>
      <AuthProvider>
        <App />
        <Toaster richColors position="bottom-right" />
      </AuthProvider>
    </BrowserRouter>
  </StrictMode>,
);
