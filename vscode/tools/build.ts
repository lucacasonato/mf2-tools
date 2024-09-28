import { standalone, js } from "./grammar.ts";

Deno.writeTextFileSync(
  new URL(import.meta.resolve("../syntaxes/mf2.tmLanguage.json")),
  JSON.stringify(standalone, null, 2)
);

Deno.writeTextFileSync(
  new URL(import.meta.resolve("../syntaxes/mf2.js.json")),
  JSON.stringify(js, null, 2)
);
