// Covers that were made (again) while Billy runs, and repairs of covers that failed to load.

import { api } from "$lib/api";

/** How many times each document's cover was made during this session, by file id. A cover
 *  shows it in its image address, so a cover made again is loaded afresh. */
export const remade = $state<Record<number, number>>({});

/** Notes that these documents' covers were (re)made. */
export function coversMade(fileIds: number[]) {
  for (const id of fileIds) remade[id] = (remade[id] ?? 0) + 1;
}

let waiting = new Map<number, (queued: boolean) => void>();
let timer: ReturnType<typeof setTimeout> | null = null;

/** Asks Billy to make again a cover whose image could not be loaded. Requests are sent
 *  together. Resolves to true when the cover file was missing and is being made again. */
export function repairCover(fileId: number): Promise<boolean> {
  return new Promise((resolve) => {
    const previous = waiting.get(fileId);
    waiting.set(fileId, (queued) => {
      previous?.(queued);
      resolve(queued);
    });
    timer ??= setTimeout(flush, 300);
  });
}

async function flush() {
  const batch = waiting;
  waiting = new Map();
  timer = null;
  let queued = 0;
  try {
    queued = await api.repairCovers([...batch.keys()]);
  } catch {
    // Leave the placeholders as they are.
  }
  for (const done of batch.values()) done(queued > 0);
}
