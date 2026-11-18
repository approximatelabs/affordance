// A video shown to a model as its frames: pictures, each sized by the style sheet.
//
//   import { frames } from "@affordance/lib/media.tsx";
//   const content = [...frames(shots, {seconds_apart: 1, max_frames: 8}), {type: "text", text: "What happens?"}];
//
// `shots` are the video's frames as pictures (data URLs, or `{src, at}` with the second each
// was taken at), in order: what `ffmpeg -i clip.mp4 -vf fps=1 frame%03d.png` writes, which a
// tool can run on the agent's machine. The result is message content: a line with each frame's
// time, then the frame. Every frame is an `<img>` in the model's format, so layout sizes it as
// it sizes any picture:
//
//   img { max-tokens: 64tok; }      /* each frame */
//
// and the video takes at most max_frames times that. Any model that takes pictures can read
// it. (`<video>` is the model's own video placeholder with the file as it is: its tokens are
// the server's to decide.)

// `max_frames` of `shots`, spread evenly from the first to the last.
function spread(shots, max_frames) {
  if (!max_frames || shots.length <= max_frames) return shots.map((shot, i) => [shot, i]);
  if (max_frames === 1) return [[shots[0], 0]];
  const picked = [];
  for (let k = 0; k < max_frames; k++) {
    const i = Math.round(k * (shots.length - 1) / (max_frames - 1));
    picked.push([shots[i], i]);
  }
  return picked;
}

// The frames as message content. `seconds_apart`: the time between one shot and the next,
// when the shots do not carry their own (`at`).
export function frames(shots, { seconds_apart = null, max_frames = null } = {}) {
  const content = [];
  for (const [shot, i] of spread(shots, max_frames)) {
    const src = typeof shot === "string" ? shot : shot.src;
    const at = typeof shot === "string" || shot.at === undefined ? (seconds_apart === null ? null : i * seconds_apart) : shot.at;
    if (at !== null) content.push({type: "text", text: `[${at.toFixed(1)} s]`});
    content.push({type: "image_url", image_url: {url: src}});
  }
  return content;
}
