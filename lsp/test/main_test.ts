import { assert, assertEquals, assertRejects } from "@std/assert";
import type { LSPTest } from "./util/mod.ts";
import { CompletionItem, CompletionList } from "vscode-languageserver-types";

let AutoLSPTest: typeof LSPTest;
if (Deno.env.get("MODE") === "wasm") {
  AutoLSPTest = (await import("./util/wasm.ts")).WasmLSPTest;
} else {
  AutoLSPTest = (await import("./util/native.ts")).NativeLSPTest;
}

Deno.test("diagnostics", async () => {
  await using lsp = new AutoLSPTest();
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
  await using lsp = new AutoLSPTest();
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
  await using lsp = new AutoLSPTest();
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
          message: "$foo has already been declared.",
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
          message: "$foo is used before it is declared.",
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
          version: 3,
          text: ".local $foo = {$foo :fn opt=$foo} {{}}",
        },
      },
    );

    const diagnostic = await diagnosticPromise;
    assertEquals(diagnostic, {
      diagnostics: [
        {
          message: "$foo is used before it is declared.",
          range: {
            start: { character: 15, line: 0 },
            end: { character: 19, line: 0 },
          },
          severity: 1,
          source: "mf2",
        },
        {
          message: "$foo is used before it is declared.",
          range: {
            start: { character: 28, line: 0 },
            end: { character: 32, line: 0 },
          },
          severity: 1,
          source: "mf2",
        },
      ],
      uri: "file:///src/main.mf2",
      version: 3,
    });
  });
});

Deno.test("variable rename", async (t) => {
  await using lsp = new AutoLSPTest();
  await lsp.initialize();

  await lsp.notify(
    "textDocument/didOpen",
    {
      textDocument: {
        uri: "file:///src/main.mf2",
        languageId: "mf2",
        version: 1,
        text: ".local $foo = {1} .local $bar = {$foo}\n\n.match $foo 1 {{}}",
      },
    },
  );

  await t.step("prepare to rename from the middle of foo", async () => {
    const response = await lsp.request("textDocument/prepareRename", {
      textDocument: { uri: "file:///src/main.mf2" },
      position: { line: 0, character: 10 },
    });

    assertEquals(response, {
      start: { character: 8, line: 0 },
      end: { character: 11, line: 0 },
    });
  });

  await t.step("prepare to rename from the $", async () => {
    const response = await lsp.request("textDocument/prepareRename", {
      textDocument: { uri: "file:///src/main.mf2" },
      position: { line: 0, character: 7 },
    });

    assertEquals(response, {
      start: { character: 8, line: 0 },
      end: { character: 11, line: 0 },
    });
  });

  await t.step("prepare to rename from the space before $", async () => {
    const response = await lsp.request("textDocument/prepareRename", {
      textDocument: { uri: "file:///src/main.mf2" },
      position: { line: 0, character: 6 },
    });

    assertEquals(response, null);
  });

  await t.step("prepare to rename from .local", async () => {
    const response = await lsp.request("textDocument/prepareRename", {
      textDocument: { uri: "file:///src/main.mf2" },
      position: { line: 0, character: 2 },
    });

    assertEquals(response, null);
  });

  await t.step("rename foo to hello, from the middle of foo", async () => {
    const response = await lsp.request("textDocument/rename", {
      textDocument: { uri: "file:///src/main.mf2" },
      position: { line: 0, character: 10 },
      newName: "hello",
    });

    assertEquals(response, {
      changes: {
        "file:///src/main.mf2": [
          {
            newText: "$hello",
            range: {
              start: { character: 7, line: 0 },
              end: { character: 11, line: 0 },
            },
          },
          {
            newText: "$hello",
            range: {
              start: { character: 33, line: 0 },
              end: { character: 37, line: 0 },
            },
          },
          {
            newText: "$hello",
            range: {
              start: { character: 7, line: 2 },
              end: { character: 11, line: 2 },
            },
          },
        ],
      },
    });
  });

  await t.step("rename the .local to hello", async () => {
    const error = await assertRejects(() =>
      lsp.request("textDocument/rename", {
        textDocument: { uri: "file:///src/main.mf2" },
        position: { line: 0, character: 2 },
        newName: "hello",
      })
    );

    assertEquals(error, {
      code: -32803,
      message: "No variable to rename at the given position.",
    });
  });

  await t.step("rename foo to 123, from the middle of foo", async () => {
    const error = await assertRejects(() =>
      lsp.request("textDocument/rename", {
        textDocument: { uri: "file:///src/main.mf2" },
        position: { line: 0, character: 10 },
        newName: "123",
      })
    );

    assertEquals(error, {
      code: -32803,
      message: "Invalid variable name.",
    });
  });
});

Deno.test("semantic tokens", async () => {
  await using lsp = new AutoLSPTest();

  await lsp.initialize();

  await lsp.notify(
    "textDocument/didOpen",
    {
      textDocument: {
        uri: "file:///src/main.mf2",
        languageId: "mf2",
        version: 1,
        text:
          ".local $a = {:x c=1}\n.local $b = {2}\n.match $a\n* {{ {|a\nb\r\nc| } }}",
      },
    },
  );

  const semanticTokens = await lsp.request("textDocument/semanticTokens/full", {
    textDocument: {
      uri: "file:///src/main.mf2",
    },
  });

  assert(semanticTokens);
  // deno-fmt-ignore
  assertEquals(semanticTokens.data, [
    0, 7, 2, 0, 0, // $a
    0, 7, 1, 2, 0, // :x
    0, 2, 1, 1, 0, // c
    0, 2, 1, 5, 0, // 1
    1, 7, 2, 0, 0, // $b
    0, 6, 1, 5, 0, // 2
    1, 0, 6, 3, 0, // .match
    0, 7, 2, 0, 0, // $a
    1, 6, 3, 4, 0, // |a\n
    1, 0, 3, 4, 0, // b\r\n
    1, 0, 2, 4, 0, // c
  ]);
});

for (const def of ["definition", "declaration"] as const) {
  Deno.test(`go to ${def}`, async (t) => {
    await using lsp = new AutoLSPTest();

    await lsp.initialize();

    await lsp.notify(
      "textDocument/didOpen",
      {
        textDocument: {
          uri: "file:///src/main.mf2",
          languageId: "mf2",
          version: 1,
          text: ".input {$bar} .local $foo = {$bar} .match $foo 1 {{}}",
        },
      },
    );

    await t.step("for .input", async () => {
      const response = await lsp.request(`textDocument/${def}`, {
        textDocument: { uri: "file:///src/main.mf2" },
        position: { line: 0, character: 31 },
      });

      assertEquals(response, {
        uri: "file:///src/main.mf2",
        range: {
          start: { line: 0, character: 8 },
          end: { line: 0, character: 12 },
        },
      });
    });

    await t.step("for .local", async () => {
      const response = await lsp.request(`textDocument/${def}`, {
        textDocument: { uri: "file:///src/main.mf2" },
        position: { line: 0, character: 38 },
      });

      assertEquals(response, null);
    });

    await t.step("somewhere else", async () => {
      const response = await lsp.request(`textDocument/${def}`, {
        textDocument: { uri: "file:///src/main.mf2" },
        position: { line: 0, character: 44 },
      });

      assertEquals(response, {
        uri: "file:///src/main.mf2",
        range: {
          start: { line: 0, character: 21 },
          end: { line: 0, character: 25 },
        },
      });
    });
  });
}

Deno.test("completions", async (t) => {
  await using lsp = new AutoLSPTest();

  await lsp.initialize();

  await lsp.notify(
    "textDocument/didOpen",
    {
      textDocument: {
        uri: "file:///src/main.mf2",
        languageId: "mf2",
        version: 1,
        text: ".local $foo = {1} .input {$bar}\n{{ {$f} {} }}",
      },
    },
  );

  function sort(response: CompletionList | CompletionItem[] | null) {
    if (Array.isArray(response)) {
      response.sort((a: { label: string }, b: { label: string }) =>
        a.label.localeCompare(b.label)
      );
    }
  }

  await t.step("completions for $f", async () => {
    const response = await lsp.request("textDocument/completion", {
      textDocument: { uri: "file:///src/main.mf2" },
      position: { line: 1, character: 6 },
    });

    sort(response);

    assertEquals(response, [
      {
        kind: 6,
        label: "$bar",
        textEdit: {
          newText: "$bar",
          range: {
            start: { line: 1, character: 4 },
            end: { line: 1, character: 6 },
          },
        },
      },
      {
        kind: 6,
        label: "$foo",
        textEdit: {
          newText: "$foo",
          range: {
            start: { line: 1, character: 4 },
            end: { line: 1, character: 6 },
          },
        },
      },
    ]);
  });

  await t.step("completions for empty variable location", async () => {
    const response = await lsp.request("textDocument/completion", {
      textDocument: { uri: "file:///src/main.mf2" },
      position: { line: 1, character: 9 },
    });

    sort(response);

    assertEquals(response, [
      {
        kind: 6,
        label: "$bar",
      },
      {
        kind: 6,
        label: "$f",
      },
      {
        kind: 6,
        label: "$foo",
      },
    ]);
  });

  await t.step("completions where no variable is allowed", async () => {
    const response = await lsp.request("textDocument/completion", {
      textDocument: { uri: "file:///src/main.mf2" },
      position: { line: 0, character: 15 },
    });

    assertEquals(response, null);
  });
});

Deno.test("formatting", async (t) => {
  await using lsp = new AutoLSPTest();

  await lsp.initialize();

  await t.step("formats valid code", async () => {
    const uri = "file:///src/test-1.mf2";

    await lsp.notify(
      "textDocument/didOpen",
      {
        textDocument: {
          uri,
          languageId: "mf2",
          version: 1,
          text: ".local $foo = {1} .input {$bar}\n{{Hello {$foo} and {$bar}!}}",
        },
      },
    );

    const res = await lsp.request("textDocument/formatting", {
      textDocument: { uri },
      options: { tabSize: 2, insertSpaces: true },
    });

    assertEquals(res, [
      {
        newText:
          ".local $foo = {1}\n.input {$bar}\n{{Hello {$foo} and {$bar}!}}\n",
        range: {
          start: { line: 0, character: 0 },
          end: { line: 1, character: 28 },
        },
      },
    ]);
  });

  await t.step("formats code with scope errors", async () => {
    const uri = "file:///src/test-2.mf2";

    await lsp.notify(
      "textDocument/didOpen",
      {
        textDocument: {
          uri,
          languageId: "mf2",
          version: 1,
          text: ".local $foo = {$bar} .input {$bar}\n{{}}",
        },
      },
    );

    const res = await lsp.request("textDocument/formatting", {
      textDocument: { uri },
      options: { tabSize: 2, insertSpaces: true },
    });

    assertEquals(res, [
      {
        newText: ".local $foo = {$bar}\n.input {$bar}\n{{}}\n",
        range: {
          start: { line: 0, character: 0 },
          end: { line: 1, character: 4 },
        },
      },
    ]);
  });

  await t.step({
    name: "formats code with recoverable syntax errors",
    ignore: true,
    fn: async () => {
      const uri = "file:///src/test-3.mf2";

      await lsp.notify(
        "textDocument/didOpen",
        {
          textDocument: {
            uri,
            languageId: "mf2",
            version: 1,
            text: "{.2}",
          },
        },
      );

      const res = await lsp.request("textDocument/formatting", {
        textDocument: { uri },
        options: { tabSize: 2, insertSpaces: true },
      });

      assertEquals(res, [
        {
          newText: "{ .2 }\n",
          range: {
            start: { line: 0, character: 0 },
            end: { line: 0, character: 4 },
          },
        },
      ]);
    },
  });

  await t.step(
    "does not format code with unrecoverable syntax errors",
    async () => {
      const uri = "file:///src/test-4.mf2";

      await lsp.notify(
        "textDocument/didOpen",
        {
          textDocument: {
            uri,
            languageId: "mf2",
            version: 1,
            text: ".hello world {    .4 }}",
          },
        },
      );

      const res = await lsp.request("textDocument/formatting", {
        textDocument: { uri },
        options: { tabSize: 2, insertSpaces: true },
      });

      assertEquals(res, null);
    },
  );
});
