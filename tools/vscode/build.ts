import { js, standalone } from "./grammar.ts";

Deno.writeTextFileSync(
  new URL(import.meta.resolve("../../vscode/syntaxes/mf2.tmLanguage.json")),
  JSON.stringify(standalone, null, 2) + "\n",
);

Deno.writeTextFileSync(
  new URL(import.meta.resolve("../../vscode/syntaxes/mf2.js.json")),
  JSON.stringify(js, null, 2) + "\n",
);
