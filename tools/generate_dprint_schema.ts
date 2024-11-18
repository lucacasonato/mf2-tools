const schema = Deno.readTextFileSync("./dprint-plugin/schema.json");
const fixed = schema.replaceAll("{{VERSION}}", Deno.args[0]);
Deno.writeTextFileSync("./dprint-plugin-mf2.schema.json", fixed);
