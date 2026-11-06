import { type Accessor, cleanup, signal } from "nsh";
import { defaultRetry, keySegments, type QueryKey, type RetryPolicy, retryDelay, type QueryStatus } from "./policy";

export interface InfiniteQueryOptions<P, T> {
  key: QueryKey;
  initialPageParam: P;
  /** The next page's param, from a page. `undefined` or `null` means that page was the last. */
  getNextPageParam: (page: T) => P | undefined | null;
  /** Enables `fetchPreviousPage`, for a list opened in the middle of its data. */
  getPreviousPageParam?: (page: T) => P | undefined | null;
  fetchPage: (context: { pageParam: P; key: string[]; signal: AbortSignal }) => Promise<T>;
  /** Keeps at most this many pages, dropping from the far end. */
  maxPages?: number;
  retry?: RetryPolicy;
}

export interface InfiniteQuery<T> {
  /** The loaded pages, in order. */
  pages: Accessor<readonly T[]>;
  status: Accessor<QueryStatus>;
  error: Accessor<unknown>;
  /** True until the first page arrives. */
  isPending: Accessor<boolean>;
  hasNextPage: Accessor<boolean>;
  hasPreviousPage: Accessor<boolean>;
  isFetchingNextPage: Accessor<boolean>;
  isFetchingPreviousPage: Accessor<boolean>;
  /** Loads the next page. Does nothing past the last one or while one is on its way, so `onLoadMore` can call it. */
  fetchNextPage(): void;
  fetchPreviousPage(): void;
  /** Drops the pages and loads from the first page again. */
  refetch(): void;
}

/** Observes a paged list and loads its first page. Owned by the calling component. */
export function createInfiniteQuery<P, T>(options: InfiniteQueryOptions<P, T>): InfiniteQuery<T> {
  const retry = options.retry ?? defaultRetry;
  const pages = signal<readonly T[]>([]);
  const status = signal<QueryStatus>("idle");
  const error = signal<unknown>(null);
  const direction = signal<"next" | "previous" | null>(null);
  const nextParam = signal<P | null | undefined>(options.initialPageParam);
  const previousParam = signal<P | null | undefined>(null);
  let request = 0;
  let controller: AbortController | null = null;

  const present = (param: P | null | undefined): param is P => param !== undefined && param !== null;

  const run = async (way: "next" | "previous", param: P): Promise<void> => {
    const id = ++request;
    controller?.abort();
    const own = new AbortController();
    controller = own;
    direction.set(way);
    error.set(null);
    status.set(pages().length === 0 ? "loading-empty" : "loading-with-data");
    let attempt = 0;
    for (;;) {
      try {
        const page = await options.fetchPage({ pageParam: param, key: keySegments(options.key), signal: own.signal });
        if (id !== request) return;
        const first = pages().length === 0;
        if (way === "next") {
          nextParam.set(options.getNextPageParam(page));
          if (first) previousParam.set(options.getPreviousPageParam?.(page));
          const grown = [...pages(), page];
          pages.set(options.maxPages ? grown.slice(-options.maxPages) : grown);
        } else {
          previousParam.set(options.getPreviousPageParam?.(page));
          const grown = [page, ...pages()];
          pages.set(options.maxPages ? grown.slice(0, options.maxPages) : grown);
        }
        status.set("success");
        direction.set(null);
        return;
      } catch (failure) {
        if (id !== request) return;
        if (attempt < retry.maxRetries) {
          const delay = retryDelay(retry, attempt);
          attempt += 1;
          await new Promise((resolve) => setTimeout(resolve, delay));
          if (id !== request) return;
          continue;
        }
        error.set(failure);
        status.set("failure");
        direction.set(null);
        return;
      }
    }
  };

  const fetchNextPage = (): void => {
    const param = nextParam();
    if (present(param) && direction() !== "next") void run("next", param);
  };

  const fetchPreviousPage = (): void => {
    const param = previousParam();
    if (present(param) && direction() !== "previous") void run("previous", param);
  };

  cleanup(() => {
    request += 1;
    controller?.abort();
  });

  fetchNextPage();

  return {
    pages,
    status,
    error,
    isPending: () => pages().length === 0 && (status() === "idle" || status() === "loading-empty"),
    hasNextPage: () => present(nextParam()),
    hasPreviousPage: () => present(previousParam()),
    isFetchingNextPage: () => direction() === "next",
    isFetchingPreviousPage: () => direction() === "previous",
    fetchNextPage,
    fetchPreviousPage,
    refetch() {
      request += 1;
      controller?.abort();
      pages.set([]);
      direction.set(null);
      nextParam.set(options.initialPageParam);
      previousParam.set(null);
      fetchNextPage();
    },
  };
}
