import grammar from "./grammar.ts";

Deno.writeTextFileSync(
  new URL(import.meta.resolve("../syntaxes/mf2.tmLanguage.json")),
  JSON.stringify(grammar, null, 2)
);
