import { realpathSync } from "node:fs";
import { fileURLToPath, URL } from "node:url";
import { mergeConfig } from "vite";

import config from "./vite.config";

// A worktree may reuse the checkout's installed dependencies. Permit that
// resolved directory, while retaining Vite's default sensitive-file denylist.
export default mergeConfig(config, {
  server: {
    fs: {
      allow: [
        fileURLToPath(new URL("../..", import.meta.url)),
        realpathSync(
          fileURLToPath(new URL("../../node_modules", import.meta.url)),
        ),
      ],
    },
  },
});
