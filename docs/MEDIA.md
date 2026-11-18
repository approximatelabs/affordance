# Pictures, sound and video

A model that takes pictures reads them as tokens, like everything else in its prompt. This
page says how an app gives a model a picture, a sound clip or a video, how a style sheet
decides how many tokens each one takes, and what is sent to the model's server.

## A picture in a prompt

A model's prompt is a list of tokens. A picture is not text, so the prompt holds placeholder
tokens where the picture belongs, and the picture's file travels with the request. The model's
server puts the picture in at the placeholders.

How many tokens a picture takes depends on the model and on the picture's size. Qwen3.5 cuts
a picture into squares of 32 by 32 pixels and each square is one token, so a picture of 640 by
480 pixels is 20 by 15 squares, 300 tokens. A 4K screenshot is over 8,000.

## A model's size rules are part of its tokenizer

Text cannot be laid out without a tokenizer. In the same way a picture, a clip or a video
cannot be laid out without the model's size rule for that kind of file: how many tokens the
model's own processor turns it into. Those rules are part of a model's tokenizer here. They
are read from the processor files the publisher ships beside `tokenizer.json`, or from the
model's row in the registry.

When a model has no rule for a kind of file, or a file cannot be measured, the file is not
laid out at a guess. Its `alt` text is laid out in its place, the file is not sent, and the
run says so. The tokens laid out are always the tokens the model reads.

## Giving a model a picture

A tool returns a picture as part of its result, or an app puts one in a message. The picture
is a data URL, which is the file's bytes written out as text.

```tsx
/** Take a picture of the screen. */
function look() {
  return [
    { type: "text", text: "The screen right now." },
    { type: "image_url", image_url: { url: "data:image/png;base64," + shot } },
  ];
}
```

A sound clip is an `input_audio` part (`{ type: "input_audio", input_audio: { data, format:
"wav" } }`) and a video file is a `video_url` part. The app does not name the model or its
placeholder: the model's format writes the placeholder, and Affordance sends the file.

A source is always the file itself, as a data URL or as base64 bytes. A web address or a file
path is refused before the request is sent, so an agent cannot make a model server fetch or
open something for it.

## Sizing a picture in the style sheet

A style sheet gives a picture a size in tokens, the way CSS gives an image a size in pixels.

```css
img { max-tokens: 256tok; }
```

With this rule every picture takes at most 256 tokens. Affordance works out the largest size
of the picture, in pixels, that the model turns into 256 tokens or fewer, resamples the file
to that size, and sends the resampled file. The model's server then produces exactly the
tokens the layout counted.

| Property | What it does |
|---|---|
| `max-tokens: 256tok` | The most tokens the picture takes. `2win` is 2% of the model's window. |
| `min-tokens: 64tok` | The fewest. A smaller picture is enlarged to reach it. |
| `object-fit: scale-down` | The default. Shrink the picture to fit; never enlarge it. |
| `object-fit: contain` | Shrink or enlarge it to the largest size that fits `max-tokens`. |
| `object-fit: none` | Keep its natural size. If that is over `max-tokens` it is not shown. |

The picture keeps its shape. Its natural size is its own size, moved onto the model's grid
and inside the smallest and largest picture the model's processor accepts.

A picture that cannot fit its box is not shown, and its file is not sent. Its `alt` text takes
its place, as on a web page. This happens when `max-tokens` is under the smallest picture the
model accepts (64 tokens on Qwen3.5), or with `object-fit: none` when the picture is too big.

### The newest picture and the older ones

An agent that takes a screenshot every step usually wants the current one sharp and the old
ones small. The newest picture in the prompt carries `last`:

```css
img        { max-tokens: 64tok; }
img[last]  { max-tokens: 1024tok; }
```

### Pictures count toward the window

Layout holds one placeholder for every token a picture takes. So a box that drops old turns,
and the check that a prompt fits the model's window, count each picture at what the model
reads.

## Sizing a sound clip

A clip's tokens follow its length. Gemma 4 gives a clip one token for each 40 milliseconds,
rounded up, and 2 more around it: one second is 27 tokens.

```css
audio { max-tokens: 250tok; }
```

A clip over its cap is trimmed, and the trimmed clip is sent. The start is kept. With
`overflow: clip-start` the end is kept instead. A cap with no room for any of the clip shows
the `alt` text.

Affordance reads and trims PCM WAV files. A clip in another format, such as MP3, is sent as
it is, and a rule that caps one is an error that asks for a WAV.

## A video as its frames

A video is shown to a model as a few of its frames, each one a picture:

```tsx
import { frames } from "@affordance/lib/media.tsx";

const content = [
  ...frames(shots, { seconds_apart: 1, max_frames: 8 }),
  { type: "text", text: "What happens in this video?" },
];
```

`shots` is the video's frames as pictures, in order. `frames` picks `max_frames` of them,
spread evenly from the first to the last, and writes the time of each one before it
(`[2.0 s]`). Each frame is a picture, so the same rules size it:

```css
img { max-tokens: 64tok; }
```

Eight frames of at most 64 tokens is a video of at most 512 tokens, plus the times. Any model
that takes pictures can read it.

Getting frames out of a video file is the app's work. A tool can run this on the agent's
machine and read the files back:

```sh
ffmpeg -i clip.mp4 -vf fps=1 frame%03d.png
```

## A video file in the model's own format

A `video_url` part gives the model the video file itself, in the model's own video format.
Qwen's models from Qwen3-VL on read a video as timed frames: the processor takes two frames a
second (at least 4, at most 768), gives each a size on its grid, and writes every two frames
as their time and their tokens:

```
<0.2 seconds><|vision_start|> 100 tokens <|vision_end|><1.4 seconds><|vision_start|> 100 tokens <|vision_end|>
```

Layout holds exactly those tokens, times included, so the video counts at what the model
reads. The file is sent as it is and is not resized: a video over its `max-tokens` is not
shown, and its alt text stands in. To give a video a size, show it as frames (above). An MP4
or MOV file is read.

## Which models

How a model turns a picture into tokens is recorded for each model, beside its prompt format
(`views/index.json`, `agent.media_metrics`). `tools/add_model.py` records it when a model is
added, from the model's own processor configuration.

- Pictures: 218 registered models. Each rule was checked against the model's own processor
  code, and most also against a hosted server's prompt token count.
  - Qwen's vision models and the models built on them, and GLM's vision models, cut a picture
    into a grid of cells.
  - Kimi-VL counts 14-pixel patches, pads each side to 28 pixels, and gives one token for each
    28 by 28 square (224 by 224 is 64 tokens, 1280 by 720 is 1,075).
  - LFM2-VL lays out a picture up to about half a megapixel as one block of 64 to 256 tokens.
    It cuts a larger one into tiles of 512 pixels with a marker each, then adds a thumbnail
    (1280 by 720 is two tiles and a thumbnail, 769 tokens).
  - Pixtral (Mistral's vision models) lays a picture out in rows of 28-pixel cells, each row
    followed by a break token and the last by an end token, with its longest side at most
    1,540 pixels.
  - PaddleOCR-VL cuts a picture into a grid of 28-pixel cells, like Qwen's.
  - Gemma 3 and Gemma 3n give every picture 256 tokens.
  - Gemma 4 gives a picture a count that follows its shape and not its size: a square is 256
    tokens, a 4:3 picture 266, a 16:9 picture 264. A picture with such a rule cannot be shrunk
    by a style rule; it is shown or it is not.
  - DeepSeek V4.1 lays a picture out in rows of 42-pixel cells, each row ended by one more
    token, between a start and an end token.
  - Llama 4, InternVL and Intern-S1, Nemotron's vision models, Command A Vision and Granite
    Vision cut a picture into tiles and add a thumbnail of the whole picture.
  - Some families have a rule of their own, each in its own file under `src/media_kinds/`, with
    a test that reproduces its processor's counts: Step3, R-4B, Youtu, GLM-5.3-Flash,
    HunyuanOCR, Inkling, Unlimited-OCR, Command A Vision, Nemotron, Muse Glimmer, MiniMax,
    Ovis, SmolVLM2, Granite Docling, Molmo2 and MiniCPM-V.
  - MiniCPM-V numbers its pictures (`<image_id>0</image_id>` before the first). Each picture
    gets its number from its place in the prompt.
- Sound: the 7 Gemma 4 models.
- A video file in the model's own format: 85 models, Qwen3-VL and after.
- Any other model, for that kind of file: the alt text.

A model whose format was written by hand for Affordance (Qwen 3.5 and 3.6, Llama, Mistral,
DeepSeek and others) gets the same treatment: its pictures, clips and videos are laid out by its
size rules and carry their files.

The same rules apply whichever app renders the prompt. A model bound by its registered name
(`template: "Qwen/Qwen3.5-4B"`) takes them from the registry. A model loaded from its own
directory takes them from the processor files beside its `tokenizer.json`.

An app can still state a count itself: `<img tok_size={n}>` is `n` placeholders and the file
as it is, and the author owns that number.

## When the count is wrong

A server that takes token ids reports how many prompt tokens it processed. Affordance
compares that with the tokens it laid out on every call. A difference is printed as a warning
and counted in the run's summary as `token_mismatches`, which should be 0. It shows a wrong
`tok_size` and a server that tokenizes a prompt again.

## What each kind of server receives

- **SGLang** (`kind: sglang`): the prompt's token ids, with one placeholder for each file, and
  the files in `image_data`, `audio_data` and `video_data`, each list in prompt order. SGLang
  expands the placeholder into the file's tokens.
- **A chat endpoint** (`kind: chat`, such as vLLM's or a hosted API): the file is a part of
  its message (`image_url`, `input_audio`, `video_url`), and the server writes the prompt.
- **A completions endpoint** (`kind: completions`) takes token ids and no files. A request
  with a file is refused with a message that names the chat kind.

## Checking a model on a real server

`tools/check-media.sh` serves one model with SGLang and shows it a blue picture, the same
picture sized to 256 tokens by a rule, a green video as four frames and as a video file, and a
clip of the word "banana", whole and trimmed. For each it checks that the model names what it was shown and
that the server counted the same number of prompt tokens as the layout did.

```sh
MODEL=/path/to/model TEMPLATE=Qwen/Qwen3.5-4B ./tools/check-media.sh
```

Measured this way on SGLang 0.5.23:

| Model | Case | Tokens laid out | Tokens the server counted |
|---|---|---|---|
| Qwen3.5-4B | a picture as it is | 370 | 370 |
| Qwen3.5-4B | the picture sized to 256 tokens | 562 | 562 |
| Qwen3.5-4B | a video as four frames | 598 | 598 |
| Qwen3.5-4B | the video file itself | 522 | 522 |
| GLM-4.6V-Flash | a picture as it is | 262 | 262 |
| GLM-4.6V-Flash | the picture sized to 256 tokens | 454 | 454 |
| GLM-4.6V-Flash | a video as four frames | 490 | 490 |
| Kimi-VL-A3B | a picture as it is | 110 | 110 |
| Kimi-VL-A3B | the picture sized to 256 tokens | 302 | 302 |
| Kimi-VL-A3B | a video as four frames | 344 | 344 |
| LFM2.5-VL-1.6B | a picture as it is | 176 | 176 |
| LFM2.5-VL-1.6B | a video as four frames | 404 | 404 |
| Gemma 4 E2B | a picture | 348 | 348 |
| Gemma 4 E2B | a video as four frames | 1,146 | 1,146 |
| Gemma 4 E2B | a sound clip | 205 | 205 |
| Gemma 4 E2B | the clip trimmed to 60 tokens | 155 | 155 |

A GLM video file had no size rule, so its alt text was shown and the check said so. The GLM rows need a one-line fix in SGLang, which stock 0.5.23 does not have: it fails on a
GLM picture sent with token ids. [The harness guide](HARNESS_GUIDE.md) says what SGLang does
with a prompt's token ids when a file comes with them.

## The element underneath

A model's format writes its pictures and clips with three elements, which an app can also
write directly (`spec/LANGUAGE.md`, section 4):

```tsx
<img   src={dataUrl} t="<|image_pad|>" metrics={metrics} alt="a chart of sales" />
<audio src={dataUrl} t="<|audio|>"     metrics={metrics} alt="a voice note" />
<video src={dataUrl} t="<|video_pad|>" metrics={metrics} alt="a screen recording" />
```

`t` is the model's placeholder token. `metrics` is the model's size rule for that kind of
file; without it the element uses the tokenizer's own. With `tok_size={n}` it is `n`
placeholders and the file as it is.
