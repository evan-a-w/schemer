(let ((x 1)) (while (< x 10) (begin (display x) (set! x (+ x 1)))))

(let ((x 1)) (begin
             (for i in '(1 2 3 4 5) (begin (display x) (set! x (+ x 1)))))
             (display #\a)
             (display x))
