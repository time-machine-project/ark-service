# ARK Service

ARK Service mints random ARKs (Archival Resource Keys) with optional NCDA check characters, validates ARKs against the ARK specification, and redirects ARKs to per-shoulder targets. The service is written in Rust with Axum and ships as a Docker image on the GitHub Container Registry.

## ARK primer

### Structure

```
ark:[/]NAAN/shoulder+blade[qualifier]
```

In `ark:12345/x6np1wh8kc/page2.pdf`:

- `12345` is the NAAN (Name Assigning Authority Number), the identifier of the assigning organization.
- `x6` is the shoulder, a namespace inside the NAAN.
- `np1wh8kc` is the blade; its last character can be a check character.
- `/page2.pdf` is the qualifier, a path to a component or variant.

The labels `ark:` and `ark:/` are equivalent.

### Shoulders

A primordinal shoulder is one or more betanumeric characters ending in the first digit after the NAAN (the first-digit convention):

```
ark:12345/x6np1wh8k    # shoulder "x6"
ark:12345/b3th89n      # shoulder "b3"
ark:12345/bcd7fgh      # shoulder "bcd7"
ark:12345/abc7fgh      # no shoulder: "a" is not betanumeric
```

No `/` belongs between shoulder and blade:

```
ark:12345/x6np1wh8k/page2.pdf   # correct
ark:12345/x6/np1wh8k/page2.pdf  # incorrect
```

### Betanumeric characters

NAANs, shoulders and minted blades use the betanumeric alphabet, the digits and the lowercase letters without vowels, `y` and `l`:

```
0123456789bcdfghjkmnpqrstvwxz
```

The alphabet has no vowels, so minted ARKs contain no words, and no `l`, which is confused with `1`. Its 29 characters give the check character algorithm a prime radix.

ARKs are case-sensitive outside the label and the NAAN: `ark:12345/x6ABC` and `ark:12345/x6abc` are different ARKs. The service mints lowercase ARKs only.

### Check characters

The Noid Check Digit Algorithm (NCDA) computes a check character over the check zone: the NAAN, `/`, shoulder and blade, without the qualifiers. For `ark:13030/xf93gt2q`, the check character `q` covers `13030/xf93gt2`.

Each character's ordinal in the betanumeric alphabet is multiplied by its position; the sum modulo 29 selects the check character. Characters outside the alphabet, uppercase letters and `/` included, have the ordinal 0, as in Noid. For betanumeric strings of up to 27 characters before the check character, NCDA detects every single-character substitution and every transposition of two adjacent characters, including those that involve the check character. A check zone also contains `/`, which shares the ordinal 0 with `0`, so swapping the two goes undetected. With a five-digit NAAN and a two-character shoulder, that covers blades of up to 19 characters before the check character.

References: [ARK specification (draft-kunze-ark-43)](https://www.ietf.org/archive/id/draft-kunze-ark-43.html), [NCDA](https://metacpan.org/dist/Noid/view/noid#NOID-CHECK-DIGIT-ALGORITHM).

### Specification conformance

The service implements [draft-kunze-ark-43](https://www.ietf.org/archive/id/draft-kunze-ark-43.html). Normalization follows the eight steps of §3.2 in order, and matching and validation use the normalized components. The service also applies the optional cleanup of §3.1 to the characters it receives: it removes whitespace and treats U+2010 to U+2015 as hyphens. In a request path these characters arrive percent-encoded, and percent-encoded octets are ARK characters (§3.1), so the cleanup does not apply to them.

Where the specification leaves a choice, the service decides as follows:

- A Name starting with a digit has that digit as its shoulder, following the definition "one or more betanumeric characters ending in a digit" (§2.4.1). A Name that does not start with a primordinal shoulder has no shoulder.
- Normalization removes no inflections (§3.2 step 7). Inflections such as `?info` travel in the query string, which step 2 removes, and resolution forwards them to the target.
- `ark://BCDFG/x6` keeps its uppercase NAAN: step 4 runs before step 8 removes the extra `/`.

`tests/conformance.rs` holds the conformance table. Each rule the service implements has an ID, each case cites the rules it exercises, and a test fails when a rule has no case. Give a new rule an ID in `RULES` and at least one case. `tests/properties.rs` checks invariants over generated ARKs: published and transcribed variants normalize to the same ARK, minted ARKs validate and parse back to their shoulder, and the check character detects every single-character substitution and adjacent transposition.

## Design

The service mints ARKs at random and stores nothing:

- It keeps no record of minted ARKs, so it detects no collisions and guarantees no uniqueness, neither between nor within requests. The blade length sets the collision risk (see [Collision risk](#collision-risk)).
- The system that uses the ARKs maps them to resources.
- Instances share nothing, so any number can run side by side.

It does not provide:

- binding metadata or URLs to individual ARKs;
- sequential or patterned minting (Noid templates such as `.rdde` or `.zeddk`);
- Noid's hold, queue, peppermint, update and fetch operations;
- metrics endpoints.

It suits projects that track ARK-to-resource mappings in their own database, mint moderate volumes, and run containerized infrastructure.

| Feature | This service | Noid |
| --- | --- | --- |
| ARK generation | Random only | Random + sequential patterns |
| Storage | None | Berkeley DB |
| Binding ARKs to URLs | No (you manage externally) | Yes (bind command) |
| Collision detection | No | Yes |
| Scaling | Horizontal (stateless) | Vertical (single DB) |
| Setup | Environment variables | Database and templates |
| Shoulders | Yes (multiple) | Yes (via templates) |
| Check characters | Yes (NCDA) | Yes (NCDA) |

### Collision risk

Random blades of length n come from 29^n possible values. The birthday bound gives the number of minted ARKs at which a collision becomes 1% likely, about √(0.02 × 29^n):

| Blade length | Possible blades  | 1% collision risk at |
| ------------ | ---------------- | -------------------- |
| 6            | ~595 million     | ~3,450 ARKs          |
| 8            | ~500 billion     | ~100,000 ARKs        |
| 10           | ~421 trillion    | ~2.9 million ARKs    |
| 12           | ~354 quadrillion | ~84 million ARKs     |

At 8 characters, 10,000 ARKs carry a collision risk of about 0.01%, 100,000 ARKs about 1%, and 1 million ARKs about 63%. Volumes beyond the 1% point need a longer blade or external collision detection.

## API reference

The examples use the default address `http://localhost:3000` and the configuration from [Shoulders](#shoulders-configuration): NAAN `12345`, shoulder `x6` with a 10-character blade and check characters, shoulder `b3` with the default 8-character blade and no check characters.

The `POST` endpoints answer a body they cannot read with a plain-text message:

- `415 Unsupported Media Type`: the header `Content-Type: application/json` is missing.
- `400 Bad Request`: the body is not JSON.
- `422 Unprocessable Entity`: the body does not match the endpoint's fields, such as a missing `shoulder` or a `count` that is not a non-negative integer.

### Health check

```
GET /ark:{naan}/servicestatus
```

```bash
curl http://localhost:3000/ark:12345/servicestatus
```

The response is `OK`.

### Service info

Returns the NAAN and the configured shoulders, sorted by shoulder. `example_ark` is a newly minted ARK on every request.

```
GET /api/v1/info
```

```json
{
  "naan": "12345",
  "shoulders": [
    {
      "shoulder": "b3",
      "project_name": "Project Beta",
      "uses_check_character": false,
      "blade_length": 8,
      "example_ark": "ark:12345/b3452jnbtf"
    },
    {
      "shoulder": "x6",
      "project_name": "Project Alpha",
      "uses_check_character": true,
      "blade_length": 10,
      "example_ark": "ark:12345/x6w0v9gt072wc"
    }
  ]
}
```

### Mint ARKs

```
POST /api/v1/mint
```

```json
{
  "shoulder": "x6",
  "count": 3
}
```

- `shoulder` (required): a configured shoulder.
- `count` (optional, default 1): the number of ARKs. A larger count than `MAX_MINT_COUNT` returns `MAX_MINT_COUNT` ARKs; `count` in the response gives the number minted.

```bash
curl -X POST http://localhost:3000/api/v1/mint \
  -H "Content-Type: application/json" \
  -d '{"shoulder": "x6", "count": 3}'
```

```json
{
  "arks": [
    "ark:12345/x6k2cnxn92jc9",
    "ark:12345/x6df013xgp7vz",
    "ark:12345/x6c2hbnkfws60"
  ],
  "count": 3
}
```

An unknown shoulder returns `404 Not Found` with the plain-text body `Shoulder not found`.

### Validate ARKs

Validates up to 1000 ARKs against the specification and returns their components.

```
POST /api/v1/validate
```

```json
{
  "arks": ["ark:12345/x6np1wh8kc"],
  "has_check_character": true
}
```

- `arks` (required): ARKs to validate, with or without an NMA in front, such as `https://n2t.net/`. More than 1000 return `400 Bad Request`.
- `has_check_character` (optional): `true` tests the last character of the base Name as an NCDA check character. Otherwise no check character is tested.

`valid` reflects only the specification: the label, a betanumeric NAAN, a Name, and characters within the ARK repertoire. An ARK from any NAAN, minted by any rules, is valid if it conforms. The other fields report:

- `naan`, `shoulder`, `blade`: the normalized components. `shoulder` is `null` when the Name does not start with a primordinal shoulder; `blade` is then the whole base Name.
- `naan_matches`: whether the NAAN is the service's NAAN.
- `shoulder_registered`: whether the shoulder is configured under the service's NAAN; `null` when the NAAN does not match. The ARK redirects when both fields are `true`.
- `has_check_character`: the value from the request, or `null`.
- `check_character_valid`: the check character result, or `null` when no check character was tested, which includes ARKs without a blade. A wrong check character does not change `valid`.
- `error`: why the ARK is not valid.
- `warnings`: any of `Blade contains non-betanumeric characters`, `NAAN does not match this resolver`, `Shoulder is not registered in this resolver` and `Check character does not match`.

`error` and an empty `warnings` are left out. An input without an ARK label, NAAN or Name, such as `doi:10.1/x`, has `null` for `naan`, `shoulder`, `blade`, `naan_matches`, `shoulder_registered` and `check_character_valid`, and `error` names the missing part.

```bash
curl -X POST http://localhost:3000/api/v1/validate \
  -H "Content-Type: application/json" \
  -d '{"arks": ["ark:12345/x6np1wh8kc"], "has_check_character": true}'
```

```json
{
  "results": [
    {
      "ark": "ark:12345/x6np1wh8kc",
      "valid": true,
      "naan": "12345",
      "shoulder": "x6",
      "blade": "np1wh8kc",
      "naan_matches": true,
      "shoulder_registered": true,
      "has_check_character": true,
      "check_character_valid": true
    }
  ]
}
```

An ARK from another NAAN with a UUID blade, validated without `has_check_character`:

```json
{
  "results": [
    {
      "ark": "ark:99999/b1550e8400-e29b-41d4-a716-446655440000",
      "valid": true,
      "naan": "99999",
      "shoulder": "b1",
      "blade": "550e8400e29b41d4a716446655440000",
      "naan_matches": false,
      "shoulder_registered": null,
      "has_check_character": null,
      "check_character_valid": null,
      "warnings": [
        "Blade contains non-betanumeric characters",
        "NAAN does not match this resolver"
      ]
    }
  ]
}
```

An ARK with a character outside the repertoire:

```json
{
  "results": [
    {
      "ark": "ark:12345/x6np,1wh8k",
      "valid": false,
      "naan": "12345",
      "shoulder": "x6",
      "blade": "np,1wh8k",
      "naan_matches": true,
      "shoulder_registered": true,
      "has_check_character": null,
      "check_character_valid": null,
      "error": "Name or qualifier contains characters outside the ARK repertoire",
      "warnings": ["Blade contains non-betanumeric characters"]
    }
  ]
}
```

### Resolve ARKs

Redirects an ARK to the target its shoulder's `route_pattern` builds.

```
GET /ark:{naan}/{shoulder}{blade}[{qualifier}]
```

The label may be `ark:` or `ark:/` in any letter case. The service selects the shoulder by the normalized NAAN and shoulder, so `ark:12345/x-6np1wh8kc` resolves like `ark:12345/x6np1wh8kc`. The target gets the ARK as received, with the label written as `ark:`; the ARK's query string, such as `?info`, becomes the target's query string. The redirect target is serialized as a URL, which percent-encodes characters a URL cannot contain.

```bash
curl -I http://localhost:3000/ark:12345/x6np1wh8kc/page2.pdf
```

```
HTTP/1.1 302 Found
Location: https://alpha.example.org/x6np1wh8kc/page2.pdf
```

Errors, each with a plain-text body:

- `400 Bad Request`: the path is not an ARK that conforms to the specification, the NAAN is not the service's, or the target's path would contain a `.` or `..` segment, encoded or not.
- `404 Not Found`: the shoulder is not configured, the Name has no primordinal shoulder, or the path does not start with an ARK label.
- `500 Internal Server Error`: the shoulder's `route_pattern` built no valid URL.

## Configuration

The service reads its configuration from environment variables. An invalid value stops the service at startup with a message naming the variable.

### NAAN

Required. The NAAN this service mints and resolves; it must be betanumeric and is used in lowercase.

```bash
export NAAN="12345"
```

### PORT

Optional, default 3000. The port the service listens on, on all interfaces.

### DEFAULT_BLADE_LENGTH

Optional, default 8, at least 1. The number of random characters in a minted blade, without the check character, for shoulders without their own `blade_length`. With a check character the blade has one more character: `DEFAULT_BLADE_LENGTH=8` gives 9-character blades.

### MAX_MINT_COUNT

Optional, default 1000, at least 1. The largest number of ARKs one mint request returns.

### RUST_LOG

Optional, default `info`. The log filter in the [`EnvFilter` syntax](https://docs.rs/tracing-subscriber/0.3/tracing_subscriber/filter/struct.EnvFilter.html), such as `warn` or `ark_service=debug`. Logs go to standard output.

### SHOULDERS configuration

Required. The shoulders and their redirect targets, as a JSON object:

```bash
export SHOULDERS='{
  "x6": {
    "route_pattern": "https://alpha.example.org/${value}",
    "project_name": "Project Alpha",
    "uses_check_character": true,
    "blade_length": 10
  },
  "b3": {
    "route_pattern": "https://beta.example.org/items/${value}",
    "project_name": "Project Beta",
    "uses_check_character": false
  }
}'
```

- `route_pattern` (required): the redirect target (see [Route patterns](#route-patterns)).
- `project_name` (required): a name for the shoulder's project.
- `uses_check_character` (optional, default `true`): whether minted ARKs end with a check character.
- `blade_length` (optional, at least 1): the shoulder's blade length without the check character; `DEFAULT_BLADE_LENGTH` applies without it.

Each key must be a primordinal shoulder, such as `x6` or `bcd7`: ARKs minted under any other key could never resolve, so such a key stops the service. Unknown fields also stop the service.

`SHOULDERS` also accepts a simple format of comma-separated entries with three tab-separated fields, shoulder, route pattern and project name. A literal `\t` counts as a tab, as Docker Compose YAML passes it. Entries in this format use check characters and `DEFAULT_BLADE_LENGTH`, and their fields cannot contain commas or tabs.

```bash
export SHOULDERS='x6\thttps://alpha.example.org/${value}\tProject Alpha,b3\thttps://beta.example.org/items/${value}\tProject Beta'
```

### Route patterns

A `route_pattern` is an `http` or `https` URL. Without template variables, the service appends the ARK to it; end such a pattern with `/` or `=`. With template variables, each variable carries its part of the ARK as received, including hyphens and letter case. Each `${var}` may also be written `{var}`. For `ark:12345/x6np1wh8k/page2.pdf`:

- `${pid}`: `ark:12345/x6np1wh8k/page2.pdf`, always with the label `ark:`
- `${scheme}`: `ark`
- `${content}`: `12345/x6np1wh8k/page2.pdf`, everything after the label
- `${prefix}` or `${naan}`: `12345`
- `${value}`: `x6np1wh8k/page2.pdf`, everything after the NAAN and `/`

```
"route_pattern": "https://example.org/${value}"
"route_pattern": "https://api.example.org/${prefix}/items/${value}"
"route_pattern": "https://resolver.example.org/resolve?id=${pid}"
```

The variables end before the ARK's query string. That query string joins the target's query, after the pattern's own parameters and an `&`: `https://example.org/${value}?format=json` turns `ark:12345/x6np1wh8k?info` into `https://example.org/x6np1wh8k?format=json&info`. The bare `?` inflection has no parameter to add, so it reaches the target only when the pattern has no query of its own.

Variables may appear in the path, query or fragment, never in the scheme, user info, host or port. In the query or fragment, `&`, `=`, `+` and `#` in a variable's value are percent-encoded, so the Name and qualifier cannot add parameters. Every `{` and `}` must belong to a known variable, and the pattern itself must contain no `.` or `..` path segment; anything else stops the service.

## Running the service

From the image:

```bash
docker run -p 3000:3000 \
  -e NAAN="12345" \
  -e SHOULDERS='{"x6":{"route_pattern":"https://example.org/${value}","project_name":"Test Project"}}' \
  ghcr.io/time-machine-project/ark-service:latest
```

From source:

```bash
export NAAN="12345"
export SHOULDERS='{"x6":{"route_pattern":"https://example.org/${value}","project_name":"Test Project"}}'

cargo run --release
```

The service stops on SIGTERM or Ctrl-C after finishing open requests.

## Releases

CI runs formatting, clippy and the tests before it builds an image. Pushes to `main` publish the tags `main` and `sha-<commit>`. A `v*.*.*` tag publishes the version tags, the major-version tag only from 1.0 on. A tag without a prerelease suffix, such as `v0.1.0` but not `v0.1.0-alpha`, also moves `latest`, so `latest` always points at a release.

The image checks its own health with `curl` against `/ark:${NAAN}/servicestatus`.
