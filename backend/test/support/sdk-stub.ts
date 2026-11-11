// A stub SDK client: records each command's class name and input, answers from a script.
import type { DocClient } from '../../src/adapters/dynamo/_shared/dynamo-call.js';

/** One recorded call. */
interface Call {
  readonly command: string;
  readonly input: unknown;
}

/** An SDK-style error (name is what adapters switch on). */
export const sdkError = (name: string, status?: number): Error =>
  Object.assign(new Error(name), {
    name,
    ...(status ? { $metadata: { httpStatusCode: status } } : {}),
  });

/** A client whose n-th send resolves `replies[n]` (an Error rejects). */
export const stubClient = (...replies: unknown[]) => {
  const calls: Call[] = [];
  const client = {
    send: (command: { constructor: { name: string }; input: unknown }) => {
      calls.push({ command: command.constructor.name, input: command.input });
      const reply = replies[calls.length - 1] ?? {};
      return reply instanceof Error ? Promise.reject(reply) : Promise.resolve(reply);
    },
  };
  return { client: client as unknown as DocClient, calls };
};
