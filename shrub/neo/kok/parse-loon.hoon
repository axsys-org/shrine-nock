=>
  |%
  ::
  ::  Like +rose except also produces line number
  ::
  ++  lily
    |*  [los=tape sab=rule]
    =+  vex=(sab [[1 1] los])
    ?~  q.vex
      [%| p=p.vex(q (dec q.p.vex))]
    ?.  =(~ q.q.u.q.vex)
      [%| p=p.vex(q (dec q.p.vex))]
    [%& p=p.u.q.vex]

  ++  make-err
    |=  [=tale:t err=wain]
    ^+  tale
    (~(put by tale) vase:slot:t [%wain err])
  ++  make-faced-slot
    |=  =term
    ^-  pith:t
    #/sys/slot/face/[term]
  ++  build
    |=  [here=pith:t =tale:t]
    ^+  tale
    ?~  src=(~(get by tale) src:slot:t)
      (make-err tale 'No src file' ~)
    ?.  ?=(%cord -.u.src) 
      (make-err tale 'Src file not cord' ~)
    =/  lon=(each loon:ford hair)  
      (parse-loon:ford here +.u.src)
    ?:  ?=(%| -.lon)
      (make-err (crip "Syntax err: line {<q.p.lon>} col {<p.p.lon>}") ~)
    =/  =crew:n
      %-  ~(gas by *crew:n)
      %-  welp
        %+  murn  pro.pile.p.lon
        |=  [face=term =stud:t]
        ^-  (unit [slot:t pith:t])
        ?@  stud  ~
        `[(make-faced-slot face) (stud-to-pith stud %pro)]
      %+  turn  vaz.pile.p.lon
      |=  [face=term =pith:t]
      ^-  [slot:t pith:t]
      [(make-faced-slot face) pith]
    %-  ~(gas by tale)
    :~  [ast:slot:t %hoon hoon.p.lon]
        [crew:slot:t %crew crew]
    ==
  --
^-  form:n
|_  [=bowl:n =saga:t]
::  handle initial
++  init
  |=  =tale:t
  ^-  (quip card:n tale:t)
  `(build here.bowl tale)
::  handle change to state
++  talk
  |=  =tale:t
  ^-  (quip card:n tale:t)
  `(build here.bowl (~(uni by q.saga) tale))
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



