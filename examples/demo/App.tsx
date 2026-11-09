import { mount, signal, Text, View } from "nsh";
import { createMutation, createQuery, queryClient } from "../../packages/nsh-query/src";

/** Stands in for a server, so the demo needs no network. */
let serverCount = 0;

function App() {
  const count = createQuery<{ n: number }>({
    key: "count",
    cache: { ttlMs: 60_000 },
    fetch: async () => ({ n: serverCount }),
  });
  const increment = createMutation<void, { n: number }>({
    invalidates: ["count"],
    mutate: async () => ({ n: ++serverCount }),
  });
  const reloads = signal(0);
  return (
    <View class="flex-1 gap-2 p-4">
      <Text>{`count ${count.data()?.n ?? "loading"}`}</Text>
      <Text role="button" onPress={() => increment.mutate()}>
        Increment on the server
      </Text>
      <Text
        role="button"
        onPress={() => {
          reloads.set(reloads() + 1);
          queryClient.invalidate("count");
        }}
      >
        {`Reload (${reloads()})`}
      </Text>
    </View>
  );
}

mount(App);
