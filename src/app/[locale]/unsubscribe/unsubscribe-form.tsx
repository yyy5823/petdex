"use client";

import { useState, useTransition } from "react";

import { resubscribeAction, unsubscribeAction } from "./actions";

type Props = {
  token: string;
  email: string;
  initiallyUnsubscribed: boolean;
};

export function UnsubscribeForm({
  token,
  email,
  initiallyUnsubscribed,
}: Props) {
  const [unsubscribed, setUnsubscribed] = useState(initiallyUnsubscribed);
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  function handleUnsubscribe() {
    setError(null);
    startTransition(async () => {
      const res = await unsubscribeAction(token);
      if (res.ok) setUnsubscribed(true);
      else setError("Could not update your preference. Try again.");
    });
  }

  function handleResubscribe() {
    setError(null);
    startTransition(async () => {
      const res = await resubscribeAction(token);
      if (res.ok) setUnsubscribed(false);
      else setError("Could not update your preference. Try again.");
    });
  }

  return (
    <div className="space-y-5 rounded-2xl border border-border-base bg-surface/76 p-6 backdrop-blur">
      <div>
        <p className="font-mono text-xs tracking-[0.22em] text-muted-3 uppercase">
          Account
        </p>
        <p className="mt-1 break-all text-sm text-muted-1">{email}</p>
      </div>

      {unsubscribed ? (
        <>
          <div>
            <p className="text-base font-semibold">
              You're unsubscribed from Petdex updates.
            </p>
            <p className="mt-2 text-sm leading-6 text-muted-2">
              You'll still get transactional notifications when your pet is
              approved, your edits are reviewed, or someone replies to your
              feedback.
            </p>
          </div>
          <button
            type="button"
            onClick={handleResubscribe}
            disabled={pending}
            className="inline-flex h-11 items-center justify-center rounded-full border border-border-base bg-transparent px-5 text-sm font-medium transition hover:bg-surface disabled:opacity-50"
          >
            {pending ? "Working…" : "Resubscribe"}
          </button>
        </>
      ) : (
        <>
          <div>
            <p className="text-base font-semibold">
              Stop getting Petdex newsletters?
            </p>
            <p className="mt-2 text-sm leading-6 text-muted-2">
              We send a few curated updates a month: new collections, pet drops,
              community moments. Transactional emails (pet approved, feedback
              replies) are not affected.
            </p>
          </div>
          <button
            type="button"
            onClick={handleUnsubscribe}
            disabled={pending}
            className="inline-flex h-11 items-center justify-center rounded-full bg-inverse px-5 text-sm font-medium text-on-inverse transition hover:bg-inverse-hover disabled:opacity-50"
          >
            {pending ? "Working…" : "Unsubscribe"}
          </button>
        </>
      )}

      {error ? (
        <p className="text-xs text-destructive" role="alert">
          {error}
        </p>
      ) : null}
    </div>
  );
}
