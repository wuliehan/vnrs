# vnrs
vnpy written in Rust

这是一个将量化框架vnpy用Rust语言重写的项目。
目前在POC（Proof of Concept）阶段，以证明将vnpy从Python语言迁移为Rust语言获得的性能提升。
vnrs通过动态加载dll文件的方式来模拟Python在运行期动态加载策略文件的功能。

测试用的策略在仓库 https://github.com/wuliehan/double_ma_strategy 中。
先说它测试的结论：回测的运算时间缩短了19倍，换言之，Rust写的vnrs运行速度是Python写的vnpy的19-20倍，提速非常明显，而运算结果除了一处有原因未知的误差，一处我没实现外，其他几十项和vnpy完全相同。

鉴于是POC项目，部分技术方案并非最佳选择，对unsafe的包装也并非十分彻底，通过非常规用法会存在Rust的safe代码不够safe的情况。
对polars的运用个人觉得其实是没有必要的，vnpy统计的单位是天，100年也就3万多天，并行运算没多少优势，反而polars大大增加了文件大小和编译时间。
如果要做正式版本，除了以上两点，不少地方会重写。
