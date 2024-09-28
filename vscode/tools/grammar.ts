// @deno-types="npm:@types/regenerate"
import regenerate from "npm:regenerate";

const s = regenerate(" ", "\r", "\t", 0x3000); // \n
const content_char = regenerate()
  .addRange(0x01, 0x08)
  .addRange(0x0b, 0x0c)
  .addRange(0x0e, 0x1f)
  .addRange(0x21, 0x2d)
  .addRange(0x2f, 0x3f)
  .addRange(0x41, 0x5b)
  .addRange(0x5d, 0x7a)
  .addRange(0x7e, 0x2fff)
  .addRange(0x3001, 0xd7ff)
  .addRange(0xe000, 0x10ffff);
// const reserved_char = regenerate(content_char, ".");
const quoted_char = regenerate(content_char, s, ".", "@", "{", "}");
const text_char = regenerate(content_char, s, ".", "@", "|");
const simple_start_char = regenerate(content_char, s, "@", "|");

const alpha = regenerate().addRange("a", "z").addRange("A", "Z");
const digit = regenerate().addRange("0", "9");

const name_start = regenerate(alpha, "_")
  .addRange(0xc0, 0xd6)
  .addRange(0xd8, 0xf6)
  .addRange(0xf8, 0x2ff)
  .addRange(0x370, 0x37d)
  .addRange(0x37f, 0x1fff)
  .addRange(0x200c, 0x200d)
  .addRange(0x2070, 0x218f)
  .addRange(0x2c00, 0x2fef)
  .addRange(0x3001, 0xd7ff)
  .addRange(0xf900, 0xfdcf)
  .addRange(0xfdf0, 0xfffc)
  .addRange(0x10000, 0xeffff);

const name_char = regenerate(name_start, digit, "-", ".", 0xb7)
  .addRange(0x300, 0x36f)
  .addRange(0x203f, 0x2040);

const name = re`${name_start}${name_char}*`;

const identifier = re`(?:${name}:)?${name}`;

const escaped_char = re`\\[\\{|}]`;

function root({
  contentName,
  begin,
  beginCaptures,
  end,
  endCaptures,
  endLookahead = end,
}: {
  contentName?: string;
  begin: string;
  beginCaptures?: object;
  end: string;
  endLookahead?: string;
  endCaptures?: object;
}) {
  return [
    {
      contentName,
      begin,
      beginCaptures,
      end,
      endCaptures,
      patterns: [
        {
          name: "simple-message.mf2",
          begin: re`(?=${simple_start_char.clone().remove(s)})`,
          end: endLookahead,
          patterns: [{ include: `source.mf2#simple-message-inner` }],
        },
        {
          name: "complex-message.mf2",
          begin: re`(?=.)(?!${s})`,
          end: endLookahead,
          patterns: [
            { include: `source.mf2#complex-message-inner` },
            {
              begin: re`(\.match)`,
              captures: {
                1: { name: "keyword.declaration.match.mf2" },
              },
              patterns: [{ include: `source.mf2#match-statement-contents` }],
              end: endLookahead,
            },
          ],
        },
      ],
    },
  ];
}

export const standalone = {
  __:
    "THIS FILE IS AUTO-GENERATED. DO NOT MODIFY MANUALLY. Run `node --run build:grammar` to regenerate it from ../tools/grammar.ts",
  $schema:
    "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
  name: "MessageFormat 2",
  patterns: root({ begin: "^", end: "(?=file)end" }),
  fileTypes: ["mf2"],
  repository: {
    "simple-message-inner": {
      patterns: [{ include: "#escaped-char" }, { include: "#placeholder" }],
    },
    "complex-message-inner": {
      patterns: [{ include: "#declaration" }, { include: "#quoted-pattern" }],
    },
    "escaped-char": {
      name: "constant.character.escape.mf2",
      match: escaped_char,
    },
    placeholder: {
      patterns: [{ include: "#expression" }, { include: "#markup" }],
    },
    expression: {
      name: "expression.mf2",
      begin: re`\{(?!${s}?[#/])`,
      patterns: [
        { include: "#variable" },
        { include: "#annotation" },
        { include: "#attribute" },
        { include: "#literal" },
      ],
      end: re`\}`,
    },
    variable: {
      name: "variable.other.mf2",
      match: re`\$(${identifier})`,
    },
    literal: {
      patterns: [
        { include: "#quoted" },
        { include: "#unquoted" },
        { include: "#number-literal" },
      ],
    },
    unquoted: {
      name: "string.unquoted.mf2",
      match: name,
    },
    quoted: {
      name: "string.quoted.mf2",
      begin: re`\|`,
      patterns: [{ match: re`${quoted_char}` }, { include: "#escaped-char" }],
      end: re`\|`,
    },
    "number-literal": {
      name: "constant.numeric.mf2",
      match: re`\b\-?(0|[1-9]${digit}*)(\.${digit}+)?([eE][-+]?${digit}+)?\b`,
    },
    attribute: {
      match: re`(@)(${identifier})(?:${s}?(=))?`,
      captures: {
        1: { name: "punctuation.definition.attribute.mf2" },
        2: { name: "entity.other.attribute-name.mf2" },
        3: { name: "punctuation.separator.key-value.mf2" },
      },
    },
    annotation: {
      patterns: [
        { include: "#function" },
        { include: "#option" },
        { include: "#private-use-annotation" },
      ],
    },
    function: {
      match: re`(:)(${identifier})`,
      captures: {
        1: { name: "punctuation.definition.function.mf2" },
        2: { name: "entity.name.function.mf2" },
      },
    },
    option: {
      match: re`(${identifier})${s}?(=)`,
      captures: {
        1: { name: "variable.parameter.function-call.mf2" },
        2: { name: "punctuation.separator.key-value.mf2" },
      },
    },
    "private-use-annotation": {
      match: "[&^]",
      name: "punctuation.definition.private-use-annotation.mf2",
    },
    markup: {
      patterns: [
        {
          begin: re`(\{${s}?#)(${identifier})`,
          beginCaptures: {
            1: { name: "punctuation.definition.markup.open.mf2" },
            2: { name: "entity.name.tag.mf2" },
          },
          end: re`/?\}`,
          endCaptures: {
            0: { name: "punctuation.definition.markup.close.mf2" },
          },
          patterns: [
            { include: "#option" },
            { include: "#attribute" },
            { include: "#literal" },
            { include: "#variable" },
          ],
        },
        {
          begin: re`(\{${s}?/)(${identifier})`,
          beginCaptures: {
            1: { name: "punctuation.definition.markup.open.mf2" },
            2: { name: "entity.name.tag.mf2" },
          },
          end: re`\}`,
          endCaptures: {
            0: { name: "punctuation.definition.markup.close.mf2" },
          },
          patterns: [
            { include: "#option" },
            { include: "#attribute" },
            { include: "#literal" },
            { include: "#variable" },
          ],
        },
      ],
    },
    declaration: {
      patterns: [
        { include: "#input-declaration" },
        { include: "#local-declaration" },
      ],
    },
    "input-declaration": {
      begin: re`(\.input)${s}?({)`,
      beginCaptures: {
        1: { name: "keyword.declaration.input.mf2" },
        2: { name: "punctuation.definition.declaration.open.mf2" },
      },
      end: re`\}`,
      endCaptures: {
        0: { name: "punctuation.definition.declaration.close.mf2" },
      },
      patterns: [
        { include: "#variable" },
        { include: "#annotation" },
        { include: "#attribute" },
        { include: "#literal" },
      ],
    },
    "local-declaration": {
      begin: re`(\.local)${s}(\$${name})${s}?=${s}?({)`,
      beginCaptures: {
        1: { name: "keyword.declaration.local.mf2" },
        2: { name: "variable.other.mf2" },
        3: { name: "punctuation.definition.declaration.open.mf2" },
      },
      end: re`\}`,
      endCaptures: {
        0: { name: "punctuation.definition.declaration.close.mf2" },
      },
      patterns: [
        { include: "#variable" },
        { include: "#annotation" },
        { include: "#attribute" },
        { include: "#literal" },
      ],
    },
    "quoted-pattern": {
      name: "quoted-pattern.mf2",
      begin: re`\{\{`,
      patterns: [
        { match: re`${text_char}` },
        { include: "#escaped-char" },
        { include: "#placeholder" },
      ],
      end: re`\}\}`,
    },
    "match-statement-contents": {
      patterns: [
        { include: "#variable" },
        { match: re`\*`, name: "punctuation.star.mf2" },
        { include: "#literal" },
        { include: "#quoted-pattern" },
      ],
    },
  },
  scopeName: "source.mf2",
};

export const js = {
  __:
    "THIS FILE IS AUTO-GENERATED. DO NOT MODIFY MANUALLY. Run `node --run build:grammar` to regenerate it from ../tools/grammar.ts",
  $schema:
    "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
  scopeName: "inline.mf2",
  injectionSelector:
    "L:(meta.embedded.block.javascript | meta.embedded.block.typescript | source.js | source.ts | source.tsx | source.vue | source.svelte | source.astro) -source.mf2 -inline.mf2 -string -comment",
  patterns: [
    {
      begin: "\\b(?:(Intl)\\??\\.)?(MessageFormat)(\\()",
      beginCaptures: {
        "1": {
          name: "entity.name.object.js",
        },
        "2": {
          name: "entity.name.function.js",
        },
        "3": {
          name: "punctuation.definition.arguments.begin.js",
        },
      },
      end: "\\)",
      endCaptures: {
        "0": {
          name: "punctuation.definition.arguments.end.js",
        },
      },
      patterns: [
        ...root({
          contentName: "meta.embedded.block.mf2",
          begin: "(`)",
          beginCaptures: {
            "0": {
              name: "string.template.js",
            },
            "1": {
              name: "punctuation.definition.string.template.begin.js",
            },
          },
          end: re`(?<!(?:^|[^\\])\\(?:\\\\)*)(${"`"})`,
          endLookahead: re`(?<!(?:^|[^\\])\\(?:\\\\)*)(?=${"`"})`,
          endCaptures: {
            "0": {
              name: "string.template.js",
            },
            "1": {
              name: "punctuation.definition.string.template.end.js",
            },
          },
        }),
        {
          patterns: [
            { include: "source.js" },
            { include: "source.ts" },
            { include: "source.js.jsx" },
            { include: "source.tsx" },
          ],
        },
      ],
    },
    ...root({
      contentName: "meta.embedded.block.mf2",
      begin: re`(/(\*)${s}?mf2${s}?(\*)/)${s}?((${"`"}))`,
      beginCaptures: {
        "1": {
          name: "comment.block.js",
        },
        "2": {
          name: "punctuation.definition.comment.js",
        },
        "3": {
          name: "punctuation.definition.comment.js",
        },
        "4": {
          name: "string.template.js",
        },
        "5": {
          name: "punctuation.definition.string.template.begin.js",
        },
      },
      end: re`(?<!(?:^|[^\\])\\(?:\\\\)*)(${"`"})`,
      endLookahead: re`(?<!(?:^|[^\\])\\(?:\\\\)*)(?=${"`"})`,
      endCaptures: {
        "0": {
          name: "string.template.js",
        },
        "1": {
          name: "punctuation.definition.string.template.end.js",
        },
      },
    }),
  ],
};

function stringifyRegenerate(range: ReturnType<typeof regenerate>) {
  return (
    "(?:" +
    range
      .toString({ hasUnicodeFlag: true })
      .replaceAll("\\u{", "\\x{")
      .replaceAll(/\\x([A-Za-z0-9]{2})/g, "\\x{$1}")
      .replaceAll(/\\u([A-Za-z0-9]{4})/g, "\\x{$1}") +
    ")"
  );
}

function re(
  array: TemplateStringsArray,
  ...values: (ReturnType<typeof regenerate> | string)[]
) {
  let result = array.raw[0];
  for (let i = 0; i < values.length; i++) {
    const val = values[i];
    if (typeof val === "object") {
      result += stringifyRegenerate(val);
    } else {
      result += values[i];
    }
    result += array.raw[i + 1];
  }
  return result;
}
