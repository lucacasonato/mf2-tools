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
          "The character 'a' can not be escaped as escape sequences can only escape '}', '{', '|', and '\\'.",
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
          "The character 'a' can not be escaped as escape sequences can only escape '}', '{', '|', and '\\'.",
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

Deno.test("scope diagnostics", async (t) => {
  await using lsp = new LSPTest();
  await lsp.initialize();

  await t.step("duplicate declaration", async () => {
    const diagnosticPromise = lsp.waitNotify("textDocument/publishDiagnostics");

    await lsp.notify(
      "textDocument/didOpen",
      {
        textDocument: {
          uri: "file:///src/main.mf2",
          languageId: "mf2",
          version: 1,
          text: ".local $foo = {1} .local $foo = {2} {{}}",
        },
      },
    );

    const diagnostic = await diagnosticPromise;
    assertEquals(diagnostic, {
      diagnostics: [
        {
          message:
            "$foo has already been declared.",
          range: {
            start: { character: 25, line: 0 },
            end: { character: 29, line: 0 },
          },
          severity: 1,
          source: "mf2",
        },
      ],
      uri: "file:///src/main.mf2",
      version: 1,
    });
  });

  await t.step("usage before declaration", async () => {
    const diagnosticPromise = lsp.waitNotify("textDocument/publishDiagnostics");

    await lsp.notify(
      "textDocument/didOpen",
      {
        textDocument: {
          uri: "file:///src/main.mf2",
          languageId: "mf2",
          version: 2,
          text: ".local $bar = {:fn a=$foo b=$asd} .input {$foo} {{}}",
        },
      },
    );

    const diagnostic = await diagnosticPromise;
    assertEquals(diagnostic, {
      diagnostics: [
        {
          message:
            "$foo is used before it is declared.",
          range: {
            start: { character: 21, line: 0 },
            end: { character: 25, line: 0 },
          },
          severity: 1,
          source: "mf2",
        },
      ],
      uri: "file:///src/main.mf2",
      version: 2,
    });
  });

  await t.step("usage in declaration", async () => {
    const diagnosticPromise = lsp.waitNotify("textDocument/publishDiagnostics");

    await lsp.notify(
      "textDocument/didOpen",
      {
        textDocument: {
          uri: "file:///src/main.mf2",
          languageId: "mf2",
          version: 2,
          text: ".local $foo = {$foo :fn opt=$foo} {{}}",
        },
      },
    );

    const diagnostic = await diagnosticPromise;
    assertEquals(diagnostic, {
      diagnostics: [
        {
          message:
            "$foo is used before it is declared.",
          range: {
            start: { character: 15, line: 0 },
            end: { character: 19, line: 0 },
          },
          severity: 1,
          source: "mf2",
        },
        {
          message:
            "$foo is used before it is declared.",
          range: {
            start: { character: 28, line: 0 },
            end: { character: 32, line: 0 },
          },
          severity: 1,
          source: "mf2",
        },
      ],
      uri: "file:///src/main.mf2",
      version: 2,
    });
  });
})
