<div align="center">
  <br></br>

  <h1>🌌 DimensionDB</h1>
  <p>
    Hobby project to learn more about database internals and a study of how well lock-free multi-dimensional linked lists work as a database foundation
  </p>
<sub>🚧 UNSTABLE 🚧</sub>

</div>

The list has several benefits over other BTree-style indexes. For example, it doesn't require rebalancing.
We also get predictable partitions suitable for distributing the database and we have some interesting memory layout possibilities with the fixed key sizes and dimensions.

## Based on

- The ebr implementation is taken from [sled](https://github.com/spacejam/sled/), along with inspiration on how to structure
lock-free data structures.
- Initial POC architecture from https://github.com/pingcap/talent-plan/
- Zachary Painter, Christina Peterson, Damian Dechev. _Lock-Free Transactional Adjacency List._
- Deli Zhang and Damian Dechev. _An efficient lock-free logarithmic search data structure based on multi-dimensional list._
