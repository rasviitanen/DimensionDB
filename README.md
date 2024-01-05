<div align="center">
  <br></br>

  <h1>🌌 DimensionDB</h1>
  <p>
    DimensionDB is a hobby database built on lock-free multi-dimensional linked lists
  </p>
<sub>🚧 UNSTABLE 🚧</sub>

</div>

Items in `DimensionDB` are stored in lock-free multi-dimensional linked lists.
This makes it possible to achieve logarithmic search performance while allowing many threads to operate on the list in parallel. Together with lock-free transactional theory (LFTT), this can handle atomic transactions in a lock-free way as well.

The list have several benefits over other BTree-style indexes. For example, it doesn't require rebalancing.
We also get predictable partitions suitable for distributing the database and we have some interesting memory layout possibilities with the fixed key sizes and dimensions.

## Based on

- The ebr implementation is taken from [sled](https://github.com/spacejam/sled/), along with inspiration on how to structure
lock-free data structures.
- Zachary Painter, Christina Peterson, Damian Dechev. _Lock-Free Transactional Adjacency List._
- Deli Zhang and Damian Dechev. _An efficient lock-free logarithmic search data structure based on multi-dimensional list._
