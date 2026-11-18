// Generic messages protocol for hosted chat APIs: turns and tools as JSON, no model tokens. A
// `chat` model binding (runtime::model::Chat) sends them to /chat/completions and writes the
// reply back as JSON (`syntax="chat"`). Records render from their fields, so drop `ids`.
export function MessagesApp({ children, system = "Use the available tools to interact with the app." }) {
  return [messagesJson(templateInput(children, system)), <span label="protocol" syntax="chat" stop={[]} in_think={false}/>];
}
