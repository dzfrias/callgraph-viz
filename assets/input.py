from collections import defaultdict


def dfs(graph, s, explored) -> set:
    stack = [s]
    connected = set()

    while stack:
        node = stack.pop()
        for neighbor in graph[node]:
            if neighbor in explored:
                continue
            stack.append(neighbor)
        explored.add(node)
        connected.add(node)
    return connected


def reverse_graph(graph):
    new_graph = defaultdict(set)

    for node, neighbors in graph.items():
        for neighbor in neighbors:
            new_graph[neighbor].add(node)

    return new_graph


def finish_order(graph) -> list:
    explored = set()
    order = []

    for s in graph:
        if s in explored:
            continue

        stack = [s]
        while stack:
            node = stack[-1]
            did_add = False
            for neighbor in graph[node]:
                if neighbor in explored:
                    continue
                stack.append(neighbor)
                did_add = True
            if not did_add:
                order.append(stack.pop())
            explored.add(node)

    return order


def find_sccs(graph) -> list[int]:
    order = finish_order(reverse_graph(graph))
    explored = set()
    sccs = []
    for node in reversed(order):
        if node in explored:
            continue
        connected = dfs(graph, node, explored)
        sccs.append(connected)

    lens = [len(scc) for scc in sccs]
    for node, neighbors in graph.items():
        if neighbors:
            continue
        lens.append(1)
    lens.sort(reverse=True)

    return lens[:5]


find_sccs({1: {2}, 2: {3}, 3: {1}, 4: {5}, 5: {6}, 6: {4}})
