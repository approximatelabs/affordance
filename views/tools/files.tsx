// File tools over a machine's shell: read, write and edit text files where the agent works.
//   const {read, write, edit} = fileTools(harness.machine.get("home"));
//   <Tool fn={read}/> <Tool fn={write}/> <Tool fn={edit}/>
// They are library code over `machine.call({cmd, stdin})`, so they work on any shell machine
// (local, SSH, a container) and an app can copy and change them. A failure throws, which a
// dispatched tool reports to the model as its error. `signal`: a function returning the
// AbortSignal a command stops on (the current turn's), if any. `permit(tool, path)`: asked before
// each call when given; a falsy answer refuses it.

// One shell word for a path.
function quote(path) {
  if (typeof path !== "string" || !path.length) throw new Error("path must be a nonempty string");
  if (/[\n\r\0]/.test(path)) throw new Error("path must be a single line");
  return "'" + path.split("'").join("'\\''") + "'";
}

export function fileTools(machine, options) {
  const signal = options && options.signal;
  const permit = options && options.permit;
  const run = async (cmd, stdin) => {
    const args = stdin === undefined ? {cmd} : {cmd, stdin};
    return await machine.call(args, {signal: signal ? signal() : undefined});
  };
  const allowed = async (tool, path) => {
    if (permit && !(await permit(tool, path))) throw new Error(`the user declined to ${tool} ${path}`);
  };
  // The whole file as text, or why it cannot be had.
  const load = async path => {
    const got = await run("cat -- " + quote(path));
    if (got.exit) throw new Error(String(got.output).trim() || `cannot read ${path} (exit ${got.exit})`);
    if (got.truncated) throw new Error(`${path} is too large to read whole; read a range of lines (offset, limit)`);
    return String(got.output);
  };
  const store = async (path, content) => {
    const file = quote(path);
    const got = await run(`mkdir -p -- "$(dirname -- ${file})" && cat > ${file}`, content);
    if (got.exit) throw new Error(String(got.output).trim() || `cannot write ${path} (exit ${got.exit})`);
  };

  /** Read a text file. `offset` (first line, from 1) and `limit` (lines) read part of a large one. */
  async function read({path, offset, limit}: {path: string; offset?: number; limit?: number}) {
    await allowed("read", path);
    if (offset === undefined && limit === undefined) return await load(path);
    const first = Math.max(1, Math.floor(offset || 1));
    const last = limit === undefined ? "$" : String(first + Math.max(1, Math.floor(limit)) - 1);
    // The file is the command's input, not an argument: no option parsing of its name, on any `sed`.
    const got = await run(`sed -n '${first},${last}p' < ${quote(path)}`);
    if (got.exit) throw new Error(String(got.output).trim() || `cannot read ${path} (exit ${got.exit})`);
    return String(got.output);
  }

  /** Write a text file with this content, replacing it if it exists. Missing directories are made. */
  async function write({path, content}: {path: string; content: string}) {
    await allowed("write", path);
    await store(path, content);
    return `wrote ${path}`;
  }

  /** Replace `old_string` with `new_string` in a file. `old_string` must match exactly once, unless `replace_all`. */
  async function edit({path, old_string, new_string, replace_all = false}: {path: string; old_string: string; new_string: string; replace_all?: boolean}) {
    await allowed("edit", path);
    if (!old_string.length) throw new Error("old_string is empty: use write to create a file");
    if (old_string === new_string) throw new Error("old_string and new_string are the same");
    const text = await load(path);
    const count = text.split(old_string).length - 1;
    if (count === 0) throw new Error(`old_string was not found in ${path}`);
    if (count > 1 && !replace_all) {
      throw new Error(`old_string matches ${count} places in ${path}: give more of the surrounding text, or set replace_all`);
    }
    await store(path, text.split(old_string).join(new_string));
    return `edited ${path} (${count} replacement${count === 1 ? "" : "s"})`;
  }

  return {read, write, edit};
}
