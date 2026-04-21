/**
 * @param {string} src
 * @returns {Record<string, string>}
 */
export function parse(src) {
  const normalized = src.replace(/\r\n/g, "\n").replace(/\r/g, "\n");

  const re =
    /^\s*(?:export\s+)?([\w.\-]+)(?:\s*=\s*?|:\s+?)(\s*'(?:\\'|[^'])*'|\s*"(?:\\"|[^"])*"|\s*`(?:\\`|[^`])*`|[^#\r\n]*?)?\s*(?:#.*)?$/gm;

  /** @type {Record<string, string>} */
  const map = Object.create(null);

  /** @type {Array<[string, boolean]>} */
  const fileEntries = [];

  let caps;
  while ((caps = re.exec(normalized)) !== null) {
    const key = caps[1] ?? "";
    let rawVal = (caps[2] ?? "").trim();

    let value;
    let firstChar = "\0";

    if (rawVal.length >= 2) {
      const first = rawVal[0];
      const last = rawVal[rawVal.length - 1];
      if ((first === "'" || first === '"' || first === "`") && first === last) {
        value = rawVal.slice(1, -1);
        firstChar = first;
      } else {
        value = rawVal;
      }
    } else {
      value = rawVal;
    }

    let finalValue;
    if (firstChar === '"') {
      finalValue = value.replace(/\\n/g, "\n").replace(/\\r/g, "\r");
    } else {
      finalValue = value;
    }

    const shouldExpand = firstChar !== "'";
    fileEntries.push([key, shouldExpand]);
    map[key] = finalValue;
  }

  for (const [key, shouldExpand] of fileEntries) {
    if (!shouldExpand) continue;

    const value = map[key];
    if (value === undefined) continue;

    const expanded = expandValue(value, map);
    if (expanded !== value) {
      map[key] = expanded;
    }
  }

  return map;
}

/**
 * @param {string} value
 * @param {Record<string, string>} map
 * @returns {string}
 */
function expandValue(value, map) {
  let result = "";
  const chars = [...value];
  const len = chars.length;
  let i = 0;

  while (i < len) {
    const ch = chars[i];

    if (ch === "\\" && i + 1 < len && chars[i + 1] === "$") {
      // Escaped $: emit literal $
      result += "$";
      i += 2;
    } else if (ch === "$") {
      i += 1;
      if (i >= len) {
        result += "$";
        break;
      }

      if (chars[i] === "{") {
        // ${VAR} or ${VAR:-default}
        i += 1;
        const keyStart = i;

        while (i < len && isAlphanumericOrUnderscore(chars[i])) {
          i += 1;
        }

        const varName = chars.slice(keyStart, i).join("");

        // Check for :-default
        let defaultValue = null;
        if (i + 1 < len && chars[i] === ":" && chars[i + 1] === "-") {
          i += 2;
          const defaultStart = i;
          while (i < len && chars[i] !== "}") {
            i += 1;
          }
          defaultValue = chars.slice(defaultStart, i).join("");
        }

        // Skip closing }
        if (i < len && chars[i] === "}") {
          i += 1;
        }

        const mapVal = map[varName];
        const resolved =
          mapVal !== undefined && mapVal !== "" ? mapVal : undefined;

        if (resolved !== undefined) {
          result += resolved;
        } else if (defaultValue !== null) {
          result += defaultValue;
        }
      } else if (isAlphaOrUnderscore(chars[i])) {
        // $VAR (no default syntax)
        const keyStart = i;
        while (i < len && isAlphanumericOrUnderscore(chars[i])) {
          i += 1;
        }
        const varName = chars.slice(keyStart, i).join("");

        const resolved = map[varName];
        result += resolved !== undefined ? resolved : "";
      } else {
        // Bare $ followed by something unexpected
        result += "$";
      }
    } else {
      result += ch;
      i += 1;
    }
  }

  return result;
}

/**
 * @param {string} ch
 * @returns {boolean}
 */
function isAlphanumericOrUnderscore(ch) {
  const c = ch.charCodeAt(0);
  return (
    (c >= 48 && c <= 57) || // 0-9
    (c >= 65 && c <= 90) || // A-Z
    (c >= 97 && c <= 122) || // a-z
    c === 95 // _
  );
}

/**
 * @param {string} ch
 * @returns {boolean}
 */
function isAlphaOrUnderscore(ch) {
  const c = ch.charCodeAt(0);
  return (
    (c >= 65 && c <= 90) || // A-Z
    (c >= 97 && c <= 122) || // a-z
    c === 95 // _
  );
}
