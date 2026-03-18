# TOON — Token-Oriented Object Notation

TOON is a compact text format for structured data.
It encodes the same data model as JSON (objects, arrays, primitives)
but uses indentation instead of braces and declares array schemas in headers.

## Why TOON

- ~40% fewer tokens than JSON (benchmarked)
- Higher LLM accuracy (74% vs 70%)  
- Human-readable at a glance
- Lossless round-trip to/from JSON

## Syntax

```
# Scalar field
key: value

# Object (indented block)
obj:
 field1: val1
 field2: val2

# Array with N items and schema header
items[N]{col1,col2,col3}:
 row1val,row2val,row3val
 row4val,row5val,row6val

# Simple list
tags[3]: swap transfer mint
```

## Atlas API

Request TOON output with either:
- `Accept: text/toon` header
- `?format=toon` query param

Content-Type of response: `text/toon; charset=utf-8`

## Example: TxFactsV1 as TOON

```
tx:
 sig:     5xZabcdef...1234
 slot:    280000000
 pos:     12
 time:    1709000000
 status:  ok
 fee:     5000
 commit:  confirmed
 compute: 45000/200000

programs[2]: TokenkegQfe... 675kPX9MHTj...

tags[2]: swap priority_fee

actions[1]{t,p,s,x,amt}:
 SWAP,X1DEX,WalletA..1234,PoolXY..5678,{"in":"1000 USDC","out":"0.5 XNT"}

tokenDeltas[2]{mint,owner,delta,dir}:
 USDC..1234,WalletA..5678,-1000000,out
 XNT..0000,WalletA..5678,500000000,in
```

## Explain + TOON for LLM Prompting

`POST /v1/tx/:sig/explain` returns `factsToon` — the full TxFactsV1 as TOON.
This is optimized for use as context in LLM prompts.

V2 TODO: the explain endpoint will accept an optional LLM provider config
and call it with factsToon as context to generate a natural-language explanation.
