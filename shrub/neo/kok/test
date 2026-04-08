:-  *spec:n
^-  form:n
|_  [=bowl:n =saga:t]
::  handle initial
++  init
  |=  =tale:t
  ^-  (quip card:n tale:t)
  ~&  %boot
  `tale
::  handle change to state
++  talk
  |=  =tale:t
  ^-  (quip card:n tale:t)
  `(~(uni by q.saga) tale)
::  handle change to children
++  take
  |=  =gift
  ^-  (quip card:n tale:t)
  `q.saga
::  handle death of dependency
++  dead
  |=  =slot:t
  ^-  (quip card:n tale:t)
  `q.saga
::  hear: handle depnendency change 
++  hear
  |=  =rely
  ^-  (quip card:n tale:t)
  `q.saga
::  goof: handle possible error
++  goof
  |=  [c=card:n err=(unit (list tank))]
  ^-  (quip card:n tale:t)
  `q.saga
--
