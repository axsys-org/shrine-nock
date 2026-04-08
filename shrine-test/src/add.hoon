!:
=>
  |%
  ++  get-dep
    |=  [=bowl:n =term]
    ^-  @
    ?~  dep=(~(get by deps.bowl) term)  0
    ?~  fil.q.u.dep  0
    =/  =tale:t  q.u.fil.q.u.dep
    =/  pail  (~(gut by tale) content:slot:t %atom 0)
    ?>  ?=([%atom @] pail)
    +.pail

  ++  build
    |=  [=bowl:n =tale:t]
    ^+  tale
    =/  lhs  (get-dep bowl %lhs)
    =/  rhs  (get-dep bowl %rhs)
    (~(put by tale) content:slot:t %atom (add rhs lhs))
  --
^-  form:n
|_  [=bowl:n =saga:t]
::  handle initial
++  init
  |=  =tale:t
  ^-  (quip card:n tale:t)
  `(build bowl tale)
::  handle change to state
++  talk
  |=  =tale:t
  ^-  (quip card:n tale:t)
  `(build bowl (~(uni by q.saga) tale))
::  handle change to children
++  take
  |=  =gift:n
  ^-  (quip card:n tale:t)
  `q.saga
::  handle death of dependency
++  dead
  |=  =slot:t
  ^-  (quip card:n tale:t)
  `q.saga
::  hear: handle depnendency change
++  hear
  |=  =rely:n
  ^-  (quip card:n tale:t)
  `(build bowl q.saga)
::  goof: handle possible error
++  goof
  |=  [c=card:n err=(unit (list tank))]
  ^-  (quip card:n tale:t)
  `q.saga
--



