(define count
   (let ((next 0))
     (lambda ()
       (let ((v next))
         (begin
             (set! next (+ next 1))
             v)))))
(count)
(count)
