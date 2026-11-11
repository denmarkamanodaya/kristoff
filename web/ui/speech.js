// Browser speech, no server and no cost: dictation through the Web Speech API
// (SpeechRecognition) and "read aloud" through speechSynthesis. Both are feature-detected.
// Note: Chrome sends dictation audio to Google's speech service, Safari to Apple's; nothing
// reaches the Kristoff backend.

const Recognition = window.SpeechRecognition || window.webkitSpeechRecognition;

/** True when this browser can dictate (Chrome, Edge, Safari; not Firefox). */
export const canDictate = typeof Recognition === 'function';
/** True when this browser can read text aloud. */
export const canSpeak = 'speechSynthesis' in window && typeof SpeechSynthesisUtterance === 'function';

/**
 * Starts live dictation.
 * @param handlers - `{ onUpdate(finalText, interimText), onEnd(), onError(code) }`; texts are the
 *   whole session so far (final) and the current guess (interim).
 * @returns `{ stop() }`.
 */
export function startDictation({ onUpdate, onEnd, onError }) {
  const rec = new Recognition();
  rec.continuous = true;
  rec.interimResults = true;
  rec.lang = navigator.language || 'en-US';
  let finalText = '';
  rec.onresult = (e) => {
    let interim = '';
    for (let i = e.resultIndex; i < e.results.length; i += 1) {
      const r = e.results[i];
      if (r.isFinal) finalText += r[0].transcript; else interim += r[0].transcript;
    }
    onUpdate(finalText, interim);
  };
  rec.onerror = (e) => onError?.(e.error);
  rec.onend = () => onEnd?.();
  rec.start();
  return { stop: () => rec.stop() };
}

let speaking = null; // the utterance in progress

/**
 * Reads text aloud, stopping anything already being read.
 * @param text - Plain text.
 * @param onEnd - Called when reading ends or is stopped.
 */
export function speak(text, onEnd) {
  stopSpeaking();
  const u = new SpeechSynthesisUtterance(text);
  u.lang = navigator.language || 'en-US';
  u.onend = () => { if (speaking === u) speaking = null; onEnd?.(); };
  u.onerror = u.onend;
  speaking = u;
  speechSynthesis.speak(u);
}

/** Stops reading aloud. */
export function stopSpeaking() {
  if (!canSpeak) return;
  speaking = null;
  speechSynthesis.cancel();
}
