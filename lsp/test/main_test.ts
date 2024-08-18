import { assertEquals } from "@std/assert";
import { LSPTest } from "./util/mod.ts";

Deno.test("diagnostics", async () => {
  await using lsp = new LSPTest();
  await lsp.initialize();

  const diagnosticPromise = lsp.waitNotify("textDocument/publishDiagnostics");

  await lsp.notify(
    "textDocument/didOpen",
    {
      textDocument: {
        uri: "file:///src/main.mf2",
        languageId: "mf2",
        version: 1,
        text: "Hello, World! \\a",
      },
    },
  );

  const diagnostic = await diagnosticPromise;
  assertEquals(diagnostic, {
    diagnostics: [
      {
        message:
          "Escape sequence can only escape '}', '{', '|', and '\\' (found 'a' at @15)",
        range: {
          end: { character: 16, line: 0 },
          start: { character: 15, line: 0 },
        },
        severity: 1,
        source: "mf2",
      },
    ],
    uri: "file:///src/main.mf2",
    version: 1,
  });
});

Deno.test("diagnostics with emoji", async () => {
  await using lsp = new LSPTest();
  await lsp.initialize();

  const diagnosticPromise = lsp.waitNotify("textDocument/publishDiagnostics");

  await lsp.notify(
    "textDocument/didOpen",
    {
      textDocument: {
        uri: "file:///src/main.mf2",
        languageId: "mf2",
        version: 1,
        text: "💭❤💞💯📘🎹⚽🍊😅🎃😻👢☂🌸⛄⭐🙈🍍☕🚚🏰👣 \\a",
      },
    },
  );

  const diagnostic = await diagnosticPromise;
  assertEquals(diagnostic, {
    diagnostics: [
      {
        message:
          "Escape sequence can only escape '}', '{', '|', and '\\' (found 'a' at @84)",
        range: {
          end: { character: 41, line: 0 },
          start: { character: 40, line: 0 },
        },
        severity: 1,
        source: "mf2",
      },
    ],
    uri: "file:///src/main.mf2",
    version: 1,
  });
});
