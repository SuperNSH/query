import { mount, signal, Text, View } from "nsh";
import { createMutation, createQuery, QueryClient, QueryClientProvider } from "../../packages/nsh-query/src";

/** Stands in for a server, so the demo needs no network. */
let serverCount = 0;

const client = new QueryClient();

function Counter() {
  const count = createQuery<{ n: number }>(() => ({
    queryKey: ["count"],
    queryFn: async () => ({ n: serverCount }),
    staleTime: 60_000,
  }));
  const increment = createMutation<{ n: number }>(() => ({
    mutationFn: async () => ({ n: ++serverCount }),
    onSuccess: () => client.invalidateQueries({ queryKey: ["count"] }),
  }));
  const reloads = signal(0);
  return (
    <View class="flex-1 gap-2 p-4">
      <Text>{`count ${count.data?.n ?? "loading"}`}</Text>
      <Text role="button" onPress={() => increment.mutate()}>
        Increment on the server
      </Text>
      <Text
        role="button"
        onPress={() => {
          reloads.set(reloads() + 1);
          void client.invalidateQueries({ queryKey: ["count"] });
        }}
      >
        {`Reload (${reloads()})`}
      </Text>
    </View>
  );
}

mount(() => (
  <QueryClientProvider client={client}>
    <Counter />
  </QueryClientProvider>
));
