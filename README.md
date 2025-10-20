# ARK Service

**A lightweight, stateless service for minting and validating ARK (Archival Resource Key) identifiers.**

ARK Service is a Rust-based web service that generates random ARK identifiers with optional NCDA check characters. Unlike traditional ARK minters (Noid, EZID), it's designed for stateless operation with no database required, making it fast and horizontally scalable. The service supports multiple "shoulders" (identifier namespaces) with customizable URL resolution patterns.

**Key Features:**

- Fast and memory-efficient (built with Rust and Axum)
- Stateless operation (no database required)
- Multiple namespace support via shoulders
- NCDA (Noid Check Digit Algorithm) for error detection
- Flexible URL resolution with template variables
- RESTful API for minting and validation
- Docker-ready with GitHub Container Registry support

## ARK Primer

### Structure

```
ark:[/]NAAN/shoulder+blade[/qualifier]
```

**Example:** `ark:12345/x6np1wh8kq/page2.pdf`

- **NAAN** (12345): Name Assigning Authority Number - your organization's identifier
- **Shoulder** (x6): Namespace prefix ending in a digit - separates projects/collections
- **Blade** (np1wh8kq): The unique identifier, optionally ending with a check character
- **Qualifier** (page2.pdf): Optional path for variants/components

Both `ark:` and `ark:/` forms are equivalent.

### Shoulders

A shoulder is a string of betanumeric characters ending in a digit (the "first-digit convention"):

```
ark:12345/x6np1wh8k    # shoulder is "x6"
ark:12345/b3th89n      # shoulder is "b3"
ark:12345/abc7defg     # shoulder is "abc7"
```

**Critical:** Never use "/" between shoulder and blade:

```
ark:12345/x6np1wh8k/page2.pdf   # correct
ark:12345/x6/np1wh8k/page2.pdf  # WRONG
```

### Betanumeric Character Set

ARKs use "betanumeric" characters - digits and consonants (excluding 'l'):

```
bcdfghjkmnpqrstvwxz0123456789
```

This avoids vowels (prevents accidental words), excludes confusable characters ('l'/'1', 'o'/'0'), and provides a prime radix (29) for the check character algorithm.

**Case sensitivity:** ARKs are technically case-sensitive, meaning `ark:12345/x6ABC` and `ark:12345/x6abc` are different identifiers. However, this service (like most ARK minters) generates only lowercase identifiers. An important quirk: uppercase and lowercase variants of the same string produce the same check character, since NCDA treats them identically for calculation purposes.

### Check Characters (NCDA)

The Noid Check Digit Algorithm appends a check character to detect transcription errors:

```
Example: ark:13030/xf93gt2q
                          ^-- check character
```

The algorithm multiplies each character's ordinal value by its position, sums them, takes modulo 29, and maps back to a betanumeric character. It guarantees detection of:

- All single character errors
- All adjacent transposition errors
- Works for identifiers < 29 characters

**Note:** Check characters protect only the base identifier (NAAN + shoulder + blade), not qualifiers.

**Learn more:** [ARK Specification (IETF)](https://www.ietf.org/archive/id/draft-kunze-ark-34.html) | [NCDA Details](https://metacpan.org/dist/Noid/view/noid#NOID-CHECK-DIGIT-ALGORITHM)

## Design Philosophy & Tradeoffs

This service differs significantly from traditional ARK minters like Noid and EZID.

### Architecture Decisions

**Stateless random generation:**

- No database or persistent storage required
- Fast, horizontally scalable, container-friendly
- **No collision detection** - suitable for moderate volumes only (see blade length guidelines)
- **No uniqueness guarantees** across service restarts
- You must manage ARK-to-resource mappings in your own system

**What's included:**

- Random identifier generation (betanumeric + optional NCDA check characters)
- Multi-shoulder namespace support
- Template-based URL resolution (302 redirects)
- Validation API for check characters and structure

**What's NOT included (vs Noid/EZID):**

- ARK binding (associating metadata/URLs with ARKs)
- Sequential/patterned minting (no `.rdde`/`.zeddk` templates)
- Persistent storage of minted ARKs
- Hold/queue/peppermint functionality
- Update/fetch operations
- Collision detection or duplicate prevention

### When to Use This Service

**Good fit:**

- You need a simple ARK minter for moderate-scale projects
- You have your own database for tracking ARK → resource mappings
- You want stateless, containerized infrastructure
- Your minting volumes align with the collision risk profiles (see configuration)

**Not a good fit:**

- You need Noid's full feature set (bind, fetch, update)
- You require guaranteed unique ARKs without external tracking
- You need sequential or patterned identifiers
- You're minting millions of ARKs and need collision detection
- You want an all-in-one resolver with metadata storage

### Comparison to Noid

| Feature               | This Service               | Noid                         |
| --------------------- | -------------------------- | ---------------------------- |
| Identifier generation | Random only                | Random + sequential patterns |
| Storage               | Stateless (no DB)          | Berkeley DB                  |
| Binding ARKs to URLs  | No (you manage externally) | Yes (bind command)           |
| Collision detection   | No                         | Yes                          |
| Scalability           | Horizontal (stateless)     | Vertical (single DB)         |
| Setup complexity      | Low (env vars)             | Medium (DB + templates)      |
| Shoulders             | Yes (multiple)             | Yes (via templates)          |
| Check characters      | Yes (NCDA)                 | Yes (NCDA)                   |

This service is essentially a stateless random ARK generator with validation - think of it as a building block you integrate into your own system, rather than a complete ARK management solution.

---

## Roadmap

This is an MVP focused on the core ARK minting functionality. Future enhancements under consideration:

**Planned features:**

- **Persistent storage backend** - Optional database support for collision detection and ARK tracking
- **Additional minting algorithms** - Sequential identifiers, custom patterns beyond random generation
- **ARK binding** - Associate metadata and URLs with minted ARKs (making it a true resolver)
- **Collision detection** - Track minted ARKs to guarantee uniqueness
- **Metrics and monitoring** - Prometheus endpoints, minting statistics, usage tracking

**Why not now?**
The current stateless design addresses the most common use case: fast, simple ARK generation for projects that manage their own ARK-to-resource mappings. Adding these features would increase complexity, so they're being considered based on real-world usage patterns and community feedback.

---

## API Reference

The ARK service provides a RESTful API for minting, validating, and resolving ARK identifiers.

### Base URL

```
http://localhost:3000
```

### Endpoints

#### 1. Health Check

Check the service status.

```
GET /ark:{naan}/servicestatus
```

**Example:**

```bash
curl http://localhost:3000/ark:12345/servicestatus
```

**Response:**

```
OK
```

#### 2. Get Service Info

Get information about the NAAN and configured shoulders.

```
GET /api/v1/info
```

**Example:**

```bash
curl http://localhost:3000/api/v1/info
```

**Response:**

```json
{
  "naan": "12345",
  "shoulders": [
    {
      "shoulder": "x6",
      "project_name": "Project Alpha",
      "uses_check_character": true,
      "blade_length": 10,
      "example_ark": "ark:12345/x6sf2qzhjgz"
    },
    {
      "shoulder": "b3",
      "project_name": "Project Beta",
      "uses_check_character": false,
      "blade_length": 8,
      "example_ark": "ark:12345/b3sf2qzhjg"
    }
  ]
}
```

#### 3. Mint ARKs

Mint one or more new ARK identifiers for a given shoulder.

```
POST /api/v1/mint
```

**Request Body:**

```json
{
  "shoulder": "x6",
  "count": 5
}
```

- `shoulder` (required): The shoulder to mint ARKs for
- `count` (optional): Number of ARKs to mint (default: 1)

**Example:**

```bash
# Mint a single ARK
curl -X POST http://localhost:3000/api/v1/mint \
  -H "Content-Type: application/json" \
  -d '{"shoulder": "x6"}'

# Mint 10 ARKs
curl -X POST http://localhost:3000/api/v1/mint \
  -H "Content-Type: application/json" \
  -d '{"shoulder": "x6", "count": 10}'
```

**Response:**

```json
{
  "count": 5,
  "arks": [
    "ark:12345/x6np1wh8kq",
    "ark:12345/x6tqb3kh8w",
    "ark:12345/x6m9zv4xp7",
    "ark:12345/x6f2hg9nk5",
    "ark:12345/x6c8dw3bt2"
  ]
}
```

**Error Response:**

```json
{
  "error": "Shoulder not found: z9"
}
```

#### 4. Validate ARKs

Validate one or more ARK identifiers and get detailed information about their components.

```
POST /api/v1/validate
```

**Request Body:**

```json
{
  "arks": ["ark:12345/x6np1wh8kq", "ark:12345/b3test123"],
  "has_check_character": true
}
```

- `arks` (required): Array of ARK identifiers to validate
- `has_check_character` (optional): Whether to validate the check character. Required for unregistered shoulders (strict mode).

**Strict Mode Behavior:**

- **Registered shoulders**: Uses the shoulder's configuration for check character validation
- **Unregistered shoulders with `has_check_character`**: Validates according to the provided hint
- **Unregistered shoulders without `has_check_character`**: Returns an error (strict mode)

**Example:**

```bash
# Validate a single ARK
curl -X POST http://localhost:3000/api/v1/validate \
  -H "Content-Type: application/json" \
  -d '{"arks": ["ark:12345/x6np1wh8kq"]}'

# Validate multiple ARKs
curl -X POST http://localhost:3000/api/v1/validate \
  -H "Content-Type: application/json" \
  -d '{"arks": ["ark:12345/x6np1wh8kq", "ark:12345/b3test123"]}'

# Validate unregistered shoulder ARK (requires has_check_character hint)
curl -X POST http://localhost:3000/api/v1/validate \
  -H "Content-Type: application/json" \
  -d '{"arks": ["ark:12345/z9custom123"], "has_check_character": true}'
```

**Response (Multiple ARKs):**

```json
{
  "results": [
    {
      "ark": "ark:12345/x6np1wh8kq",
      "valid": true,
      "naan": "12345",
      "shoulder": "x6",
      "blade": "np1wh8kq",
      "shoulder_registered": true,
      "has_check_character": true,
      "check_character_valid": true
    },
    {
      "ark": "ark:12345/b3test123",
      "valid": true,
      "naan": "12345",
      "shoulder": "b3",
      "blade": "test123",
      "shoulder_registered": true,
      "has_check_character": false,
      "check_character_valid": true
    }
  ]
}
```

**Response (Invalid ARK):**

```json
{
  "results": [
    {
      "ark": "ark:12345/x6np1wh8k",
      "valid": false,
      "naan": "12345",
      "shoulder": "x6",
      "blade": "np1wh8k",
      "shoulder_registered": true,
      "has_check_character": true,
      "check_character_valid": false,
      "warnings": [
        "Check character validation failed. Either there's an error or this ARK has no check character."
      ]
    }
  ]
}
```

**Response (Unregistered Shoulder - Strict Mode):**

```json
{
  "results": [
    {
      "ark": "ark:12345/z9custom123",
      "valid": false,
      "naan": "12345",
      "shoulder": "z9",
      "blade": "custom123",
      "shoulder_registered": false,
      "has_check_character": null,
      "check_character_valid": null,
      "error": "Unknown shoulder. Please specify has_check_character parameter to validate unregistered shoulders."
    }
  ]
}
```

#### 5. Resolve ARK

Resolve an ARK identifier to its target URL. Returns a 302 redirect.

```
GET /ark:{naan}/{shoulder}{blade}[/{qualifier}]
```

**Examples:**

```bash
# Resolve ARK without qualifier
curl -L http://localhost:3000/ark:12345/x6np1wh8kq

# Resolve ARK with qualifier
curl -L http://localhost:3000/ark:12345/x6np1wh8kq/page2.pdf

# Resolve ARK with complex qualifier path
curl -L http://localhost:3000/ark:12345/x6np1wh8kq/documents/chapter3/figure5.jpg

# Get redirect location without following (use -I for HEAD request)
curl -I http://localhost:3000/ark:12345/x6np1wh8kq
```

**Response:**

```
HTTP/1.1 302 Found
Location: https://example.org/x6np1wh8kq
```

The `-L` flag in curl will automatically follow the redirect to the target URL.

**Error Responses:**

- `404 Not Found`: Shoulder not configured
- `400 Bad Request`: Invalid ARK format or NAAN mismatch

### Configuration

The service is configured via environment variables:

**NAAN** (optional, default: "12345")

```bash
export NAAN="12345"
```

**DEFAULT_BLADE_LENGTH** (optional, default: 8)

The default length of the randomly generated blade portion of minted ARKs, **excluding the check character**. This controls how many betanumeric characters are generated. If `uses_check_character` is true, the check character will be appended after these characters, making the total blade length one character longer. Individual shoulders can override this with their own `blade_length` configuration.

For example, with `DEFAULT_BLADE_LENGTH=8` and `uses_check_character=true`, the resulting blade will be 9 characters (8 random + 1 check).

```bash
export DEFAULT_BLADE_LENGTH="8"
```

**MAX_MINT_COUNT** (optional, default: 1000)

The maximum number of ARKs that can be minted in a single request. This limit is enforced for safety to prevent accidental mass generation of identifiers.

```bash
export MAX_MINT_COUNT="1000"
```

**Collision Implications:**

The blade length determines the size of your identifier namespace and affects collision probability when minting random ARKs. With 29 betanumeric characters, the total namespace size is 29^n.

| Blade Length | Namespace Size   | Safe Minting Qty (≤1% collision risk) | Notes                                     |
| ------------ | ---------------- | ------------------------------------- | ----------------------------------------- |
| 6            | ~594 million     | ~3,450 ARKs                           | Small projects only                       |
| 8            | ~500 billion     | ~100,000 ARKs                         | **Default - suitable for most use cases** |
| 10           | ~420 trillion    | ~2.9 million ARKs                     | Large institutional collections           |
| 12           | ~354 quadrillion | ~84 million ARKs                      | Very large scale, minimal collision risk  |

**Guidelines for choosing blade length:**

- **6 characters**: Only for small pilots or testing (thousands of ARKs)
- **8 characters**: Recommended default for most institutions (up to ~100k ARKs safely)
- **10 characters**: Large institutions with millions of objects
- **12+ characters**: Extreme scale operations or when you need virtually no collision risk

**Note on collision probability:**

These estimates use the birthday paradox: collision probability becomes significant (~1%) when you've minted approximately sqrt(0.02 × N) identifiers, where N is the namespace size. The actual risk depends on your minting volume:

- At 8 characters, minting 10,000 ARKs ≈ 0.01% collision risk
- At 8 characters, minting 100,000 ARKs ≈ 1% collision risk
- At 8 characters, minting 1 million ARKs ≈ 63% collision risk (not recommended)

**Collision detection:** This service does not currently implement collision detection or maintain a database of minted ARKs. For production use with high minting volumes, consider implementing external collision detection or using sequential identifiers instead of random generation.

**SHOULDERS** (required) - JSON format:

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

**Shoulder Configuration Fields:**

- `route_pattern` (required): URL template for resolving ARKs (see Template Variables section below)
- `project_name` (required): Human-readable name for the project
- `uses_check_character` (optional, default: true): Whether to append a check character to minted ARKs
- `blade_length` (optional): Override the default blade length for this specific shoulder, **excluding the check character**. Allows different shoulders to use different identifier lengths based on their scale needs. If not specified, uses `DEFAULT_BLADE_LENGTH`. The actual minted blade will be one character longer if `uses_check_character` is true.

**SHOULDERS** - Simple format (tab-delimited):

```bash
export SHOULDERS="x6\thttps://alpha.example.org/\${value}\tProject Alpha,b3\thttps://beta.example.org/items/\${value}\tProject Beta"
```

#### Template Variables in Route Patterns

The `route_pattern` field supports template variables for flexible URL construction. Both `${var}` and `{var}` syntax are supported and equivalent.

**Available variables:**

- `${pid}` or `{pid}` - Full ARK identifier (e.g., `ark:12345/x6np1wh8k/page2.pdf`)
- `${scheme}` or `{scheme}` - Scheme (always `ark`)
- `${content}` or `{content}` - Everything after "ark:" (e.g., `12345/x6np1wh8k/page2.pdf`)
- `${prefix}` or `{prefix}` or `{naan}` - NAAN (e.g., `12345`)
- `${value}` or `{value}` - shoulder+blade+qualifier (e.g., `x6np1wh8k/page2.pdf`)

**Examples:**

```bash
# Both syntaxes are equivalent:
"route_pattern": "https://example.org/${value}"
"route_pattern": "https://example.org/{value}"

# You can mix formats:
"route_pattern": "https://api.org/${prefix}/items/{value}"

# Use as query parameter:
"route_pattern": "https://resolver.org/resolve?id=${value}"
```

**Note:** If no template variables are present in the route pattern, the full ARK identifier will be appended to the URL (N2T.net standard behavior).

### Running the Service

```bash
# Set configuration
export NAAN="12345"
export DEFAULT_BLADE_LENGTH="8"
export MAX_MINT_COUNT="1000"
export SHOULDERS='{"x6":{"route_pattern":"https://example.org/${value}","project_name":"Test Project","uses_check_character":true}}'

# Run the service
cargo run

# Or with release optimizations
cargo run --release
```

The service will start on `http://0.0.0.0:3000`.

**Example with custom blade lengths:**

```bash
# Set default blade length to 12 characters
export DEFAULT_BLADE_LENGTH="12"

# Configure shoulders with different blade lengths
export SHOULDERS='{
  "x6": {
    "route_pattern": "https://example.org/${value}",
    "project_name": "Small Project",
    "uses_check_character": true,
    "blade_length": 6
  },
  "b3": {
    "route_pattern": "https://example.org/${value}",
    "project_name": "Large Project",
    "uses_check_character": true
  }
}'

# x6 will mint 6-character ARKs, b3 will use the default (12 characters)
cargo run
```
