// A tool is a function the model can call: <Tool fn={lookup}/> offers `lookup` on the page. Its
// name is the function's (a declaration, or the const it is bound to), its description its JSDoc,
// and its arguments' schema its props parameter's type (schemaOf: string, number, boolean,
// arrays, object types, string-literal unions, T | null, type aliases). `name`, `description` and
// `schema` override them; children are the tool's anchor on the screen (none: it is only in the
// tool list). The handler's return value is the tool's result. It lowers to <aff>.
//
//   /** Look something up. */
//   async function lookup({ q }: { q: string }) { ... }
//   <Tool fn={lookup}/>
export function Tool({ fn, name, description, schema, children, ...props }) {
  const n = name || nameOf(fn);
  if (!n) throw new Error("<Tool fn>: an anonymous function needs name=");
  return <aff {...props} tool_name={n} description={description || docOf(fn) || ""} args_schema={schema || schemaOf(fn)} onRun={fn}>{children || ""}</aff>;
}
