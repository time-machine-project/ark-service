# ARK Service

**A lightweight, high-performance service for minting, validating, and resolving ARK (Archival Resource Key) identifiers.**

ARK Service is a Rust-based web service that implements the ARK identifier specification (IETF draft-kunze-ark-34). It provides a RESTful API for creating persistent, globally unique identifiers with built-in error detection through the Noid Check Digit Algorithm. The service supports multiple "shoulders" (identifier namespaces) with customizable routing patterns, making it ideal for institutions that need to manage digital object identifiers across different projects or collections.

**Key Features:**

- Fast and memory-efficient (built with Rust and Axum)
- Multiple namespace support via shoulders
- NCDA (Noid Check Digit Algorithm) for error detection
- Flexible URL resolution with template variables
- RESTful API for minting and validation
- Docker-ready with GitHub Container Registry support
- Stateless operation (no database required for basic minting)

---

## ARK Structure

### **Basic Format**

```
[https://NMA/]ark:[/]NAAN/Name[Qualifier]

Components:
- NMA: Name Mapping Authority (hostname, optional and mutable)
- ark: or ark:/ - the ARK label
- NAAN: Name Assigning Authority Number (required, immutable)
- Name: The assigned identifier (required, immutable)
- Qualifier: Optional extensions (mutable)
```

**Modern vs Classic Forms:**
ARKs can appear in two equivalent forms: modern (ark:) and classic (ark:/), differing only by the slash. These forms are considered identical in perpetuity, and resolvers should accept both.

Example:

```
https://example.org/ark:12345/x54xz321/page2.pdf
https://example.org/ark:/12345/x54xz321/page2.pdf  # equivalent
```

## NAAN (Name Assigning Authority Number)

NAANs are opaque strings of one or more betanumeric characters. Since 2001, every assigned NAAN has consisted of exactly five digits. Implementations must support a minimum NAAN length of 16 octets.

## Shoulders

A primordinal shoulder is a sequence of one or more betanumeric characters ending in a digit. This is the "first-digit convention" where the shoulder is all letters after the NAAN up to and including the first digit encountered.

Examples:

```
ark:12345/x6np1wh8k    # shoulder is "x6"
ark:12345/b3th89n      # shoulder is "b3"
ark:12345/abc7defg     # shoulder is "abc7"
```

**Critical rule:** Do not use any delimiter (especially "/") between the shoulder string and blade string, as "/" declares an object boundary.

```
✓ ark:12345/x6np1wh8k/c2/s4.pdf   # correct
✗ ark:12345/x6/np1wh8k/c2/s4.pdf  # WRONG - "/" after shoulder
```

**Unlimited shoulders:** With primordinal convention, you get infinite potential shoulders: b3, c3, d3, ... bb3, bc3, bd3, ... bbb3, etc.

## Character Set: Betanumeric

The betanumeric character set consists of digits and consonants minus the letter 'l' (ell):

```
bcdfghjkmnpqrstvwxz0123456789
```

**Why betanumeric?**

1. Avoids vowels → prevents accidental word formation
2. Excludes 'l' (ell) → prevents confusion with '1' (one)
3. Excludes 'o' (oh) → prevents confusion with '0' (zero)
4. Prime radix of R=29 → enables strong check character algorithm

**Case sensitivity:** ARKs distinguish between lower and upper case letters, which makes shorter identifiers possible (52 vs 26 letters per character position). However, the "ARK way" is to use lowercase only unless you need shorter ARKs.

## Allowed Characters

You can use digits, letters (ASCII, no diacritics), and the following characters: = @ \* + , \_ $ . - ! ~ ' ( ) %

## Identity-Inert Hyphens

Hyphens may appear but are identity inert, meaning strings that differ only by hyphens are considered identical:

```
ark:12345/141e86dc-d396-4e59-bbc2-4c3bf5326152
ark:12345/141e86dcd3964e59bbc24c3bf5326152
# These identify the same thing
```

This protects against text formatting processes that routinely introduce hyphens.

## NCDA: Noid Check Digit Algorithm

The Noid Check Digit Algorithm (NCDA) computes a check character that is appended to the tip of the blade (the last character of the base identifier). It guarantees the base identifier against the most common transcription errors: transposition of two adjacent characters and single character errors.

**Note on terminology**: The algorithm is called "Check **Digit** Algorithm" for historical reasons, but it actually produces a "check **character**" since the result can be a letter (like 'q') or a digit (like '2').

### **Algorithm Details:**

NCDA uses a prime radix of R=29 (the betanumeric repertoire) and guarantees detection of single-character and transposition errors for strings less than R=29 characters in length.

**Implementation**:

```
Step 1: Convert each betanumeric character to its ordinal value:
        0-9 → 0-9
        bcdfghjkmnpqrstvwxz → 10-28 (in that order)

Step 2: Multiply each character's ordinal value by its position
        (starting at position 1) and sum the products.

Example: 13030/xf93gt2
  char:  1   3   0   3   0   /   x   f   9   3   g   t   2
  ord:   1   3   0   3   0   0  27  13   9   3  14  24   2
  pos:   1   2   3   4   5   6   7   8   9  10  11  12  13
  prod:  1 + 6 + 0 +12 + 0 + 0+189+104+81+30+154+288+26 = 891

Step 3: The check character is determined by taking the sum modulo 29
        and finding the character at that ordinal position.

        891 mod 29 = 21
        Character with ordinal 21 = 'q'

Result: 13030/xf93gt2q (with check character appended)
```

**What it protects:**

- ✅ All single character errors
- ✅ All adjacent transposition errors
- ✅ Works for strings < 29 characters
- ❌ Does NOT protect qualifiers (the parts after the base identifier)

**Example of protected portion:**

```
https://example.org/ark:13030/tqb3kh8w/chap3/fig5.jpg
                            \________/
                        check character protects this
```

**References:**

- [NOID Check Digit Algorithm specification](https://metacpan.org/dist/Noid/view/noid#NOID-CHECK-DIGIT-ALGORITHM)
- [ARK Specification (IETF)](https://www.ietf.org/archive/id/draft-kunze-ark-34.html)

## ARK Inflections

An inflection is a change to the ending of an identifier to express a shift in meaning. Adding '?' to an ARK requests metadata without defining a separate identifier.

```
ark:12345/x54xz321        # → object
ark:12345/x54xz321?       # → metadata
ark:12345/x54xz321??      # → commitment statement
```

## Qualifiers (Extensions)

After the base identifier, you can add qualifiers to identify parts or components of the main object:

```
ark:12345/x54xz321/page2.pdf
ark:12345/x54xz321/chapter3/figure5
                  \_______________/
                     qualifiers
```

**Common uses for qualifiers:**

Qualifiers primarily express "part of" relationships, identifying components, versions, or manifestations of the main object:

- **Pages and sections**: `ark:12345/x8rd9/page5`, `ark:12345/x8rd9/chapter3`
- **File formats**: `ark:12345/x8rd9/thumbnail.jpg`, `ark:12345/x8rd9/fullres.tif`
- **Versions**: `ark:12345/x8rd9/v2`, `ark:12345/x8rd9/2024-01-15`
- **Metadata views**: `ark:12345/x8rd9/metadata.xml`, `ark:12345/x8rd9/dc`
- **Hierarchical parts**: `ark:12345/x8rd9/volume2/section3/figure12`

The base ARK (`ark:12345/x54xz321`) identifies the primary object, while qualifiers identify subordinate parts. This allows a single ARK to serve as the root for an entire hierarchy of related resources.

**Important notes:**

- Check characters are not expected to cover qualifiers, which often name subobjects that are less stable than the parent object.
- Qualifiers are mutable and can be added or changed without affecting the identity of the base ARK.
- The "/" character in qualifiers creates natural hierarchy and expresses containment relationships.

## Opacity Recommendations

Semantic opaqueness in the Name part is strongly encouraged to reduce vulnerability to era- and language-specific change. Names that look more or less like numbers avoid common problems. Mixing in betanumerics achieves a denser namespace.

## Summary Table

| Component       | Rules                             | Example               |
| --------------- | --------------------------------- | --------------------- |
| NAAN            | 5 digits (currently), betanumeric | 12345                 |
| Shoulder        | Primordinal (ends with digit)     | x6, b3, abc7          |
| Blade           | Betanumeric + check character     | np1wh8k (+ check)     |
| Characters      | Betanumeric preferred             | bcdfg...0-9           |
| Check character | NCDA algorithm, mod 29            | Last char of blade    |
| Hyphens         | Identity-inert                    | Can add/remove freely |
| Case            | Sensitive but lowercase preferred | Use lowercase         |

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

---

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

---

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

---

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

---

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

---

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

---

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
