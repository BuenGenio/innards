// Pages Function: every /api/* request on the site is handled by the billing
// worker module, so the site and billing share one deploy and one domain.
import worker from "../../../billing/src/index";
import type { Env } from "../../../billing/src/index";

export const onRequest: PagesFunction<Env> = (ctx) => worker.fetch(ctx.request, ctx.env);
